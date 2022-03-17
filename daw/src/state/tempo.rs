use core::time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, TimeSig};
use crate::widgets::SwitchState;

use druid::{Data, Lens};

#[derive(Clone, Lens)]
pub struct Tempo {
    pub current_time: Ms,
    pub current_bar: Bars,
    pub current_beat: Beats,
    start_time: Ms,
    pub play: SwitchState,
    rec: SwitchState,
    pub looping: SwitchState,
    pub bpm: Bpm,
    pub ppqn: Ppqn,
    pub sample_rate: SampleHz,
    pub time_signature: TimeSig,
}

impl Data for Tempo {
    fn same(&self, other: &Self) -> bool {
        self.current_time == other.current_time
            && self.current_bar == other.current_bar
            && self.current_beat == other.current_beat
            && self.start_time == other.start_time
            && self.play == other.play
            && self.rec == other.rec
            && self.looping == other.looping
            && self.bpm == other.bpm
            && self.time_signature.top == other.time_signature.top
            && self.time_signature.bottom == other.time_signature.bottom
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            play: SwitchState::default(),
            current_time: Ms(0.),
            current_bar: Bars(0),
            current_beat: Beats(1),
            start_time: Ms(0.),
            rec: SwitchState::default(),
            looping: SwitchState::default(),
            bpm: 120.,
            sample_rate: 44_100.0,
            // ppqn: 19_200,
            ppqn: 8,
            time_signature: TimeSig { top: 4, bottom: 4 },
        }
    }
}
