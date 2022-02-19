use time_calc::Ticks;

pub type ClipNoteId = usize;

/// A note in a clip.
#[derive(Clone, Debug)]
pub struct ClipNote {
    // An id that should be unique inside a clip
    pub id: ClipNoteId,
    // Frequency of the note
    pub midi: f32,
    // Duration in Ticks
    pub dur: Ticks,
    // Velocity
    pub vel: u8,
}

impl ClipNote {
    pub fn new(id: ClipNoteId, midi: f32, dur: Ticks, vel: u8) -> Self {
        Self { id, midi, dur, vel }
    }
}
