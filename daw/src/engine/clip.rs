use druid::{Data, im::{OrdMap, Vector, vector}};
use time_calc::Ticks;
use super::{data::*, Transport};

#[derive(Clone, Debug, Data)]
pub struct ClipNote {
    pub midi: f32,
    #[data(same_fn = "ticks")]
    pub dur: Ticks,
}

impl ClipNote {
    pub fn new(midi: f32, dur: Ticks) -> Self {
        Self { midi, dur }
    }
}

#[derive(Debug, Clone, Data)]
pub struct Clip {
    notes: OrdMap<Ticks, Vector<ClipNote>>,
    #[data(same_fn = "ticks")]
    duration: Ticks,
}

impl Clip {
    pub fn new(duration: Ticks) -> Self {
        Self {
            notes: OrdMap::new(),
            duration,
        }
    }

    pub fn add_note(&mut self, note: ClipNote, position: Ticks) {
        if self.notes.contains_key(&position) {
            self.notes.get_mut(&position).unwrap().push_back(note);
        } else {
            self.notes.insert(position, vector![note]);
        }
    }

    pub fn get_next_notes(&self, transport: &Transport) -> Option<&Vector<ClipNote>> {
        self.notes.get(&transport.current_position)
    }
}
