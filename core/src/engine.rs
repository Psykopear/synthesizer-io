// Copyright 2018 The Synthesizer IO Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Interface for the audio engine.

use crate::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use crate::id_allocator::IdAllocator;
use crate::module::Module;
use crate::modules;
use crate::queue::{Receiver, Sender};
use time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, Ticks, TimeSig};

/// Type used to identify nodes in the external interface (not to be confused
/// with nodes in the low-level graph).
pub type NodeId = usize;

/// The type of a module to be instantiated. It's not clear this should be
/// an enum, but it should do for now.
pub enum ModuleType {
    Sin,
    Saw,
}

#[derive(PartialEq)]
pub struct Track {
    id: usize,
}

impl Track {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

#[derive(Clone, PartialEq)]
pub struct Transport {
    pub current_time: Ms,
    pub current_bar: Bars,
    pub current_beat: Beats,
    start_time: Ms,
    pub playing: bool,
    pub recording: bool,
    pub looping: Option<(Ticks, Ticks)>,
    pub bpm: Bpm,
    pub ppqn: Ppqn,
    pub sample_rate: SampleHz,
    pub time_signature: TimeSig,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            playing: false,
            current_time: Ms(0.),
            current_bar: Bars(0),
            current_beat: Beats(1),
            start_time: Ms(0.),
            recording: false,
            looping: None,
            bpm: 120.,
            sample_rate: 44_100.0,
            // ppqn: 19_200,
            ppqn: 8,
            time_signature: TimeSig { top: 4, bottom: 4 },
        }
    }
}

/// The interface from the application to the audio engine.
///
/// It doesn't do the synthesis itself; the Worker (running in a real time
/// thread) handles that, but this module is responsible for driving
/// that process by sending messages.
pub struct Engine {
    sample_rate: f32,
    rx: Receiver<Message>,
    tx: Sender<Message>,

    transport: Transport,

    id_alloc: IdAllocator,

    monitor_queues: Option<MonitorQueues>,

    tracks: Vec<Track>,
}

#[derive(Clone)]
pub struct NoteEvent {
    pub down: bool,
    pub note: u8,
    pub velocity: u8,
}

struct MonitorQueues {
    rx: Receiver<Vec<f32>>,
    tx: Sender<Vec<f32>>,
}

impl Engine {
    pub fn new(sample_rate: f32, rx: Receiver<Message>, tx: Sender<Message>) -> Engine {
        let mut id_alloc = IdAllocator::new();
        id_alloc.reserve(0);
        let monitor_queues = None;
        Engine {
            sample_rate,
            rx,
            tx,
            id_alloc,
            monitor_queues,
            tracks: vec![],
            transport: Transport::default(),
        }
    }

    pub fn add_track(&mut self) -> usize {
        let track_id = self.create_node(modules::Sum::new(), [], []);
        let track = Track::new(track_id);
        self.tracks.push(track);
        self.update_master();
        track_id
    }

    pub fn set_track_node<B: IntoBoxedSlice<(usize, usize)>>(&mut self, track_id: usize, wiring: B) {
        let track = Box::new(modules::Sum::new());
        self.send_node(Node::create(track, track_id, wiring, []));
        self.update_master();
    }

    pub fn create_node<
        B1: IntoBoxedSlice<(usize, usize)>,
        B2: IntoBoxedSlice<(usize, usize)>,
        M: Module + 'static,
    >(
        &mut self,
        module: M,
        in_buf_wiring: B1,
        in_ctrl_wiring: B2,
    ) -> usize {
        let id = self.id_alloc.alloc();
        self.send_node(Node::create(
            Box::new(module),
            id,
            in_buf_wiring,
            in_ctrl_wiring,
        ));
        id
    }

    pub fn send_note_on(&mut self, ixs: Vec<usize>, midi_num: f32, velocity: f32) {
        self.tx.send(Message::Note(Note {
            ixs: ixs.into_boxed_slice(),
            midi_num,
            velocity,
            on: true,
            timestamp: 0,
        }));
    }

    pub fn send_note_off(&mut self, ixs: Vec<usize>, midi_num: f32) {
        self.tx.send(Message::Note(Note {
            ixs: ixs.into_boxed_slice(),
            midi_num,
            velocity: 0.,
            on: false,
            timestamp: 0,
        }));
    }

    pub fn remove_track(&mut self, ix: usize) {
        self.tracks.swap_remove(
            self.tracks
                .iter()
                .position(|x| x.id == ix)
                .expect("Existing track id"),
        );
    }

    fn send(&self, msg: Message) {
        self.tx.send(msg);
    }

    fn send_node(&mut self, node: Node) {
        self.send(Message::Node(node));
    }

    fn poll_rx(&mut self) -> usize {
        self.rx.recv().count()
    }

    fn poll_monitor(&self) -> Vec<f32> {
        let mut result = Vec::new();
        if let Some(ref qs) = self.monitor_queues {
            for mut item in qs.rx.recv_items() {
                result.extend_from_slice(&item);
                item.clear();
                qs.tx.send_item(item);
            }
        }
        result
    }

    fn update_master(&mut self) {
        let module = Box::new(modules::Sum::new());
        let buf_wiring: Vec<_> = self.tracks.iter().map(|n| (n.id, 0)).collect();
        self.send_node(Node::create(module, 0, buf_wiring, []));
    }

    fn instantiate_module(&mut self, ty: ModuleType) -> usize {
        let ll_id = match ty {
            ModuleType::Sin => {
                let pitch = self.create_node(modules::SmoothCtrl::new(440.0f32.log2()), [], []);
                let sample_rate = self.sample_rate;
                self.create_node(modules::Sin::new(sample_rate), [], [(pitch, 0)])
            }
            ModuleType::Saw => {
                let pitch = self.create_node(modules::SmoothCtrl::new(440.0f32.log2()), [], []);
                let sample_rate = self.sample_rate;
                self.create_node(modules::Saw::new(sample_rate), [], [(pitch, 0)])
            }
        };
        ll_id
    }
}
