use std::collections::BTreeMap;

use time_calc::Ticks;

use super::{clip::Clip, note::ClipNote};

pub type TrackId = usize;

#[derive(Debug)]
pub struct Track {
    pub id: TrackId,
    pub clips: BTreeMap<Ticks, Clip>,
    pub control: Vec<usize>,
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

    pub fn add_note(&mut self, clip_id: usize, note: ClipNote, position: Ticks) {
        if let Some((_, clip)) = self.clips.iter_mut().find(|(_, c)| *c.id() == clip_id) {
            clip.add_note(note, position);
        }
    }

    pub fn set_control(&mut self, control: Vec<usize>) {
        self.control = control;
    }

    fn get_active_clip(&self, cur_position: &Ticks) -> Option<&Clip> {
        for (position, clip) in &self.clips {
            if position <= &cur_position {
                return Some(&clip);
            }
        }
        return None;
    }

    pub fn get_notes(&self, cur_position: &Ticks) -> Option<&Vec<ClipNote>> {
        self.get_active_clip(cur_position)
            .map_or(None, |clip| clip.get_notes(&cur_position))
    }
}
