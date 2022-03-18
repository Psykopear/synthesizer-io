use core::time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, TimeSig};
use super::switch::SwitchState;

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
