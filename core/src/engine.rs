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

use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};
use std::time::Duration;

use crate::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use crate::id_allocator::IdAllocator;
use crate::module::Module;
use crate::modules;
use crate::queue::{Receiver, Sender};
use time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, Ticks, TimeSig};

/// Type used to identify nodes in the external interface (not to be confused
/// with nodes in the low-level graph).
pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct ClipNote {
    midi: f32,
    dur: Ticks,
}

impl ClipNote {
    pub fn new(midi: f32, dur: Ticks) -> Self {
        Self { midi, dur }
    }
}

#[derive(Debug)]
pub struct Clip {
    notes: BTreeMap<Ticks, Vec<ClipNote>>,
    duration: Ticks,
}

impl Clip {
    pub fn new(duration: Ticks) -> Self {
        Self {
            notes: BTreeMap::new(),
            duration,
        }
    }

    pub fn add_note(&mut self, note: ClipNote, position: Ticks) {
        if self.notes.contains_key(&position) {
            self.notes.get_mut(&position).unwrap().push(note);
        } else {
            self.notes.insert(position, vec![note]);
        }
    }

    pub fn get_next_notes(&self, transport: &Transport) -> Option<&Vec<ClipNote>> {
        self.notes.get(&transport.current_position)
    }
}

#[derive(Debug)]
pub struct Track {
    id: usize,
    clips: BTreeMap<Ticks, Clip>,
    control: Vec<usize>,
}

impl Track {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            clips: BTreeMap::new(),
            control: vec![].into(),
        }
    }

    pub fn add_clip(&mut self, position: Ticks, clip: Clip) {
        self.clips.insert(position, clip);
    }

    pub fn set_control(&mut self, control: Vec<usize>) {
        self.control = control;
    }

    fn get_active_clip(&self, transport: &Transport) -> Option<&Clip> {
        for (position, clip) in &self.clips {
            if position <= &transport.current_position {
                return Some(&clip);
            }
        }
        return None;
    }

    pub fn get_notes(&self, transport: &Transport) -> Option<&Vec<ClipNote>> {
        self.get_active_clip(transport)
            .map_or(None, |clip| clip.get_next_notes(&transport))
    }
}

#[derive(Clone, PartialEq)]
pub struct Transport {
    pub current_position: Ticks,
    pub prev_position: Ticks,
    pub start_time: Option<u128>,

    pub playing: bool,
    pub recording: bool,
    pub looping: Option<(Ticks, Ticks)>,

    pub time_signature: TimeSig,
    pub bpm: Bpm,
    pub ppqn: Ppqn,

    pub sample_rate: SampleHz,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            start_time: None,
            current_position: Ticks(1),
            prev_position: Ticks(1),
            playing: false,
            recording: false,
            looping: None,
            bpm: 120.,
            sample_rate: 48_000.0,
            // ppqn: 19_200,
            ppqn: 8,
            time_signature: TimeSig { top: 4, bottom: 4 },
        }
    }
}

impl Transport {
    pub fn new(sample_rate: SampleHz) -> Self {
        let mut transport = Self::default();
        transport.sample_rate = sample_rate;
        transport
    }

    pub fn handle(&mut self, ts: u128) {
        if self.playing && self.start_time.is_none() {
            self.start_time = Some(ts);
        }
        // Set start_time if just stopped
        if !self.playing && self.start_time.is_some() {
            self.start_time = None;
        }
        // dbg!(self.start_time);
        if self.playing {
            // Update position
            let millis = (ts - self.start_time.unwrap()) / 1000000;
            self.prev_position = self.current_position;
            self.current_position = Ms(millis as f64).to_ticks(self.bpm, self.ppqn) + Ticks(1);
            // if self.prev_position != self.current_position {
            //     dbg!(self.current_position);
            // }
            // dbg!(self.current_position.bars(self.time_signature, self.ppqn));
            if let Some((start, end)) = self.looping {
                if self.current_position >= end {
                    self.prev_position = self.current_position;
                    self.current_position = start;
                    // self.prev_position = start;
                    self.start_time = Some(ts);
                }
            }
        }
    }
}

/// The interface from the application to the audio engine.
///
/// It doesn't do the synthesis itself; the Worker (running in a real time
/// thread) handles that, but this module is responsible for driving
/// that process by sending messages.
pub struct Engine {
    rx: Receiver<Message>,
    tx: Sender<Message>,
    control_rx: Receiver<u128>,
    transport: Transport,
    id_alloc: IdAllocator,
    tracks: Vec<Track>,
}

#[derive(Clone)]
pub struct NoteEvent {
    pub down: bool,
    pub note: u8,
    pub velocity: u8,
}

impl Engine {
    pub fn new(
        sample_rate: f32,
        rx: Receiver<Message>,
        tx: Sender<Message>,
        control_rx: Receiver<u128>,
    ) -> Engine {
        let mut id_alloc = IdAllocator::new();
        // Master track
        id_alloc.reserve(0);
        Engine {
            rx,
            tx,
            control_rx,
            id_alloc,
            tracks: vec![],
            transport: Transport::new(sample_rate as f64),
        }
    }

    pub fn run(&mut self) {
        let mut events = vec![];
        loop {
            if let Some(_ts) = self.control_rx.recv_items().last() {
                let ts = *_ts;
                self.transport.handle(ts);

                if self.transport.playing
                    && self.transport.current_position != self.transport.prev_position
                {
                    for track in &self.tracks {
                        // let ixs = track.control.clone();
                        if let Some(notes) = track.get_notes(&self.transport) {
                            dbg!("Sending note", notes, self.transport.current_position);
                            for note in notes {
                                let ixs = track.control.to_vec();
                                self.tx.send(Message::Note(Note {
                                    ixs: ixs.into_boxed_slice(),
                                    midi_num: note.midi,
                                    velocity: 100.,
                                    on: true,
                                    timestamp: *_ts,
                                }));
                                let ixs = track.control.to_vec();
                                events.push(Note {
                                    ixs: ixs.into_boxed_slice(),
                                    midi_num: note.midi,
                                    velocity: 0.,
                                    on: false,
                                    timestamp: *_ts
                                        + (note
                                            .dur
                                            .to_ms(self.transport.bpm, self.transport.ppqn)
                                            .0) as u128,
                                });
                            }
                        }
                    }

                    // Consume queued events
                    let mut i = 0;
                    while i < events.len() {
                        if *_ts >= events[i].timestamp {
                            let note = events.remove(i);
                            self.tx.send(Message::Note(note));
                        } else {
                            i += 1;
                        }
                    }
                }
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    pub fn set_loop(&mut self, start: Ticks, end: Ticks) {
        self.transport.looping = Some((start, end));
    }

    pub fn play(&mut self) {
        self.transport.playing = true;
    }

    pub fn add_track(&mut self) -> usize {
        let track_id = self.create_node(modules::Sum::new(), [], []);
        let track = Track::new(track_id);
        self.tracks.push(track);
        self.update_master();
        track_id
    }

    pub fn add_clip_to_track(&mut self, track_id: usize, clip: Clip, position: Ticks) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.add_clip(position, clip);
        }
    }

    pub fn set_track_node<B1: IntoBoxedSlice<(usize, usize)>>(
        &mut self,
        track_id: usize,
        in_buf_wiring: B1,
        in_ctrl_wiring: Vec<usize>,
    ) {
        self.tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .unwrap()
            .set_control(in_ctrl_wiring);
        let track = Box::new(modules::Sum::new());
        self.send_node(Node::create(track, track_id, in_buf_wiring, []));
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

    pub fn set_param(&mut self, param: SetParam) {
        self.send(Message::SetParam(param));
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

    fn update_master(&mut self) {
        let master_track = Box::new(modules::Sum::new());
        let buf_wiring: Vec<_> = self.tracks.iter().map(|n| (n.id, 0)).collect();
        self.send_node(Node::create(master_track, 0, buf_wiring, []));
    }
}
