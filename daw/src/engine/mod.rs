//! Interface for the audio engine.
pub mod clip;
mod data;
mod track;
mod transport;

use clip::Clip;
use track::Track;
use transport::Transport;

use core::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use core::id_allocator::IdAllocator;
use core::module::Module;
use core::modules;
use core::queue::{Receiver, Sender};

use druid::im::vector;
use druid::im::Vector;
use druid::{Data, Lens};

use std::sync::Arc;
use std::time::Duration;

use time_calc::Ticks;

/// The interface from the application to the audio engine.
///
/// It doesn't do the synthesis itself; the Worker (running in a real time
/// thread) handles that, but this module is responsible for driving
/// that process by sending messages.
#[derive(Data, Clone, Lens)]
pub struct Engine {
    #[data(ignore)]
    rx: Arc<Receiver<Message>>,
    #[data(ignore)]
    tx: Arc<Sender<Message>>,
    #[data(ignore)]
    control_rx: Arc<Receiver<u128>>,
    #[data(ignore)]
    id_alloc: Arc<IdAllocator>,
    #[data(ignore)]
    events: Vector<Note>,

    pub transport: Transport,
    tracks: Vector<Track>,
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
            events: vector![],
            rx: Arc::new(rx),
            tx: Arc::new(tx),
            control_rx: Arc::new(control_rx),
            id_alloc: Arc::new(id_alloc),
            tracks: vector![],
            transport: Transport::new(sample_rate as f64),
        }
    }

    pub fn run_step(&mut self, ts: u128) {
        if !self.transport.playing {
            return;
        };

        // if self.transport.prev_position.is_some()
        //     && self.transport.current_position == self.transport.prev_position.unwrap()
        // {
        //     return;
        // }

        // println!("Handling step");
        for track in &self.tracks {
            if let Some(notes) = track.get_notes(&self.transport) {
                dbg!("Sending note", notes, self.transport.current_position);
                for note in notes {
                    self.tx.send(Message::Note(Note {
                        ixs: track.controls().into_boxed_slice(),
                        midi_num: note.midi,
                        velocity: 100.,
                        on: true,
                        timestamp: ts,
                    }));
                    self.events.push_back(Note {
                        ixs: track.controls().into_boxed_slice(),
                        midi_num: note.midi,
                        velocity: 0.,
                        on: false,
                        timestamp: ts
                            + (note.dur.to_ms(self.transport.bpm, self.transport.ppqn).0) as u128,
                    });
                }
            }
        }

        // Consume queued events
        let mut i = 0;
        while i < self.events.len() {
            if ts >= self.events[i].timestamp {
                let note = self.events.remove(i);
                self.tx.send(Message::Note(note));
            } else {
                i += 1;
            }
        }
        self.transport.handle(ts);
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
        self.tracks.push_back(track);
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
        in_ctrl_wiring: Vector<usize>,
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
        let id = Arc::make_mut(&mut self.id_alloc).alloc();
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
        self.tracks.remove(
            self.tracks
                .iter()
                .position(|x| x.id == ix)
                .expect("Existing track id"),
        );
    }

    pub fn init(&mut self) {
        self.update_master();
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
