//! Interface for the audio engine.
pub mod clip;
pub mod message;
pub mod note;
pub mod tempo;
pub mod track;

use crate::engine::note::ClipNote;
use crate::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use crate::id_allocator::IdAllocator;
use crate::module::Module;
use crate::modules;
use crate::queue::{Queue, Receiver, Sender};
use ringbuf::Consumer;
use time_calc::{Bars, Ticks};

use self::clip::{Clip, ClipId};
use self::tempo::Tempo;
use self::track::{Track, TrackId};

/// The interface from the application to the audio engine.
///
/// It doesn't do the synthesis itself; the Worker (running in a real time
/// thread) handles that, but this module is responsible for driving
/// that process by sending messages.
pub struct Engine {
    rx: Receiver<Message>,
    tx: Sender<Message>,

    // Receiver for timestamp from realtime thread.
    // Used for synchronization.
    ts_rx: Consumer<u128>,

    id_alloc: IdAllocator,

    pub tempo: Tempo,
    tracks: Vec<Track>,
    events: Vec<Note>,
}

impl Engine {
    pub fn new(
        sample_rate: f32,
        rx: Receiver<Message>,
        tx: Sender<Message>,
        ts_rx: Consumer<u128>,
    ) -> Engine {
        let mut id_alloc = IdAllocator::new();
        // Master track
        id_alloc.reserve(0);

        // UI Communication
        // let (sender, ui_rx) = Queue::new();
        // let (ui_tx, receiver) = Queue::new();
        Engine {
            rx,
            tx,
            ts_rx,
            id_alloc,
            tempo: Tempo::new(sample_rate as f64),
            tracks: vec![],
            events: vec![],
        }
    }

    pub fn run_step(&mut self) -> Option<u128> {
        // We might have received 0 or more messages.
        // If we received 0 messages, we stop here.
        // If we receive more than one, we only consider the latest.
        let ts = self.ts_rx.pop();
        if ts.is_none() {
            return None;
        }
        let ts = ts.unwrap();

        if !self.tempo.playing {
            if self.tempo.start_time.is_some() {
                self.tempo.start_time = None;
            }
            return Some(ts);
        } else if self.tempo.start_time.is_none() {
            self.tempo.start_time = Some(ts);
        };

        if self.tempo.prev_position.is_none()
            || self.tempo.current_position != self.tempo.prev_position.unwrap()
        {
            for track in &self.tracks {
                for note in track.get_notes(
                    &self.tempo.prev_position.unwrap_or(Ticks(0)),
                    &self.tempo.current_position,
                ) {
                    println!("Got note {}", note.midi);
                    let ixs = track.control.to_vec();
                    self.tx.send(Message::Note(Note {
                        ixs: ixs.into_boxed_slice(),
                        midi_num: note.midi,
                        velocity: note.vel as f32,
                        on: true,
                        timestamp: ts,
                    }));
                    let ixs = track.control.to_vec();
                    self.events.push(Note {
                        ixs: ixs.into_boxed_slice(),
                        midi_num: note.midi,
                        velocity: 0.,
                        on: false,
                        timestamp: ts + (note.dur.to_ms(self.tempo.bpm, self.tempo.ppqn).0) as u128,
                    });
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
        }
        self.tempo.step(ts);
        Some(ts)
    }

    pub fn set_loop(&mut self, start: Ticks, end: Ticks) {
        self.tempo.looping = Some((start, end));
    }

    pub fn set_play(&mut self) {
        self.tempo.playing = true;
    }

    pub fn set_pause(&mut self) {
        self.tempo.playing = false;
    }

    pub fn add_track(&mut self) -> TrackId {
        let track_id = self.create_node(modules::Sum::new(), [], []);
        let track = Track::new(track_id);
        self.tracks.push(track);
        self.update_master();
        track_id
    }

    pub fn add_clip_to_track(&mut self, track_id: TrackId, position: Ticks) -> ClipId {
        let id = self.id_alloc.alloc();
        let clip = Clip::new(
            id,
            Bars(1).to_ticks(self.tempo.time_signature, self.tempo.ppqn),
        );
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.add_clip(position, clip);
        }
        id
    }

    pub fn add_note(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        note: ClipNote,
        position: Ticks,
    ) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.add_note(clip_id, note, position);
        }
    }

    pub fn set_track_node<B1: IntoBoxedSlice<(usize, usize)>>(
        &mut self,
        track_id: TrackId,
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

    pub fn send_note_on(&self, ixs: Vec<usize>, midi_num: f32, velocity: f32) {
        self.tx.send(Message::Note(Note {
            ixs: ixs.into_boxed_slice(),
            midi_num,
            velocity,
            on: true,
            timestamp: 0,
        }));
    }

    pub fn send_note_off(&self, ixs: Vec<usize>, midi_num: f32) {
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

    pub fn remove_track(&mut self, ix: TrackId) {
        self.tracks.swap_remove(
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
        // Allocations
        let master_track = Box::new(modules::Sum::new());
        let buf_wiring: Vec<_> = self.tracks.iter().map(|n| (n.id, 0)).collect();
        self.send_node(Node::create(master_track, 0, buf_wiring, []));
    }
}
