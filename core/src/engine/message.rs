use time_calc::Ticks;

use crate::{
    graph::{IntoBoxedSlice, SetParam},
    module::Module,
};

use super::{clip::ClipId, note::ClipNote, track::TrackId};

pub enum ModuleKind {
    Adsr,
    Biquad,
    Buzz,
    ConstCtrl,
    Gain,
    Monitor,
    Pitch,
    Saw,
    Sin,
    SmoothCtrl,
    Sum,
}

pub enum Message
{
    // To the engine
    Init,
    SetLoop(bool),
    SetPlay(bool),
    SetRec(bool),
    AddTrack,
    RemoveTrack(usize),
    AddClip {
        track: TrackId,
        position: Ticks,
    },
    AddNote {
        track: TrackId,
        clip: ClipId,
        note: ClipNote,
        position: Ticks,
    },
    SetTrackDevice {
        track: TrackId,
        in_buf_wiring: Vec<usize>,
        in_ctrl_wiring: Vec<usize>,
    },
    CreateNode {
        module: ModuleKind,
        in_buf_wiring: Vec<usize>,
        in_ctrl_wiring: Vec<usize>,
    },
    NoteOn {
        ixs: Vec<usize>,
        midi_num: f32,
        vel: f32,
    },
    NoteOff {
        ixs: Vec<usize>,
        midi_num: f32,
    },
    SetParam(SetParam),
    // From the engine
    TrackAdded(TrackId),
    ClipAdded(ClipId),
    NodeCreated(usize),
}
