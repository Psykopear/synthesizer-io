use crate::widgets::SwitchState;
use core::time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, TimeSig};

use druid::{Data, Lens};

#[derive(Clone, Lens, Data)]
pub struct Tempo {
    last_ts: u128,
    current_ts: u128,
    // engine_tx: Sender<Message>,
    #[data(same_fn = "PartialEq::eq")]
    pub current_time: Ms,
    // #[data(same_fn = "PartialEq::eq")]
    pub current_bar: f64,
    // #[data(same_fn = "PartialEq::eq")]
    pub current_beat: f64,

    pub play: SwitchState,
    pub rec: SwitchState,
    pub looping: SwitchState,

    #[data(same_fn = "PartialEq::eq")]
    pub bpm: Bpm,
    #[data(same_fn = "PartialEq::eq")]
    pub ppqn: Ppqn,
    #[data(same_fn = "PartialEq::eq")]
    pub sample_rate: SampleHz,
    #[data(same_fn = "PartialEq::eq")]
    pub time_signature: TimeSig,
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            play: SwitchState::default(),
            current_time: Ms(0.),
            // current_bar: Bars(1),
            // current_beat: Beats(1),
            current_bar: 1.,
            current_beat: 1.,
            last_ts: 0,
            current_ts: 0,
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

impl Tempo {
    fn set_current(&mut self) {
        // self.current_bar = Bars(self.current_time.bars(self.bpm, self.time_signature) as i64);
        // self.current_beat =
        //     Beats(self.current_time.beats(self.bpm) as i64 % self.time_signature.bottom as i64);
        self.current_bar = self.current_time.bars(self.bpm, self.time_signature).floor() + 1.;
        self.current_beat = (self.current_time.beats(self.bpm).floor() as u64 % self.time_signature.bottom as u64) as f64;
    }

    pub fn step(&mut self, ts: u128) {
        self.last_ts = self.current_ts;
        self.current_ts = ts;
        if !self.play.on {
            return;
        }
        let delta = self.current_ts - self.last_ts;
        self.current_time += Ms(delta as f64 / 1_000_000.);
        self.set_current();
    }

    pub fn stop(&mut self) {
        self.play.on = false;
        self.current_time = Ms(0.);
        self.set_current();
    }
}
