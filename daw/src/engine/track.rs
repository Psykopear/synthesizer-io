use druid::{Data, im::{OrdMap, Vector}};
use time_calc::Ticks;

use super::{clip::{Clip, ClipNote}, transport::Transport};

#[derive(Debug, Clone, Data)]
pub struct Track {
    pub id: usize,
    clips: OrdMap<Ticks, Clip>,
    control: Vector<usize>,
}

impl Track {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            clips: OrdMap::new(),
            control: vec![].into(),
        }
    }

    pub fn controls(&self) -> Vec<usize> {
        self.control.iter().cloned().collect()
    }

    pub fn add_clip(&mut self, position: Ticks, clip: Clip) {
        self.clips.insert(position, clip);
    }

    pub fn set_control(&mut self, control: Vector<usize>) {
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

    pub fn get_notes(&self, transport: &Transport) -> Option<&Vector<ClipNote>> {
        self.get_active_clip(transport)
            .map_or(None, |clip| clip.get_next_notes(&transport))
    }
}
