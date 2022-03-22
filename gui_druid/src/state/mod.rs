use druid::{Data, Lens};

use self::tempo::Tempo;

pub mod tempo;
#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub tempo: Tempo,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tempo: Default::default(),
        }
    }
}
