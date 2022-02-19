use std::collections::BTreeMap;

use time_calc::Ticks;

use super::note::ClipNote;

pub type ClipId = usize;

/// A structure representing a Clip.
///
/// Clip notes are kept in an ordered map using the position in Ticks as the key.
/// Given a position in Ticks, the clip will return the notes that should play at
/// that point in time.
///
/// The position is relative to the start of the clip, the offset, Ticks(0) by default.
/// A duration should always be set, and any note with a position greater than duration
/// will never be played.
#[derive(Debug)]
pub struct Clip {
    id: ClipId,
    notes: BTreeMap<Ticks, Vec<ClipNote>>,
    dur: Ticks,
    offset: Ticks,
}

impl Clip {
    pub fn new(id: ClipId, dur: Ticks) -> Self {
        Self {
            id,
            notes: BTreeMap::new(),
            dur,
            offset: Ticks(0),
        }
    }

    pub fn id(&self) -> &ClipId {
        &self.id
    }

    pub fn add_note(&mut self, note: ClipNote, position: Ticks) {
        if self.notes.contains_key(&position) {
            self.notes.get_mut(&position).unwrap().push(note);
        } else {
            self.notes.insert(position, vec![note]);
        }
    }

    pub fn remove_note(&mut self, note: ClipNote, position: Ticks) {
        if self.notes.contains_key(&position) {
            self.notes.get_mut(&position).unwrap().push(note);
        } else {
            self.notes.insert(position, vec![note]);
        }
    }

    pub fn get_notes(&self, position: &Ticks) -> Option<&Vec<ClipNote>> {
        self.notes.get(&position)
    }

    pub fn get_notes_range(&self, start: &Ticks, end: &Ticks) -> Vec<&ClipNote> {
        self.notes
            .range(start..end)
            .map(|(pos, notes)| notes)
            .flatten()
            .collect()
    }

    pub fn set_duration(&mut self, dur: Ticks) {
        self.dur = dur;
    }

    pub fn set_offset(&mut self, offset: Ticks) {
        self.offset = offset;
    }

    pub fn set_all(&mut self, dur: Ticks, offset: Ticks) {
        self.offset = offset;
        self.dur = dur;
    }
}
