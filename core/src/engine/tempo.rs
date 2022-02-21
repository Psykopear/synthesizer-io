use time_calc::{Bars, Beats, Bpm, Ms, Ppqn, SampleHz, Ticks, TimeSig};

#[derive(Clone, PartialEq)]
pub struct Tempo {
    pub current_position: Ticks,
    pub prev_position: Option<Ticks>,
    pub start_time: Option<u128>,

    pub playing: bool,
    pub recording: bool,
    pub looping: Option<(Ticks, Ticks)>,

    pub time_signature: TimeSig,
    pub bpm: Bpm,
    pub ppqn: Ppqn,

    pub sample_rate: SampleHz,
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            start_time: None,
            current_position: Ticks(0),
            prev_position: None,
            playing: false,
            recording: false,
            looping: None,
            bpm: 120.,
            sample_rate: 48_000.0,
            // Part (or ticks) Per Quarter Notes, ppqn
            // Zrythm uses 960 here, 1920 seems to be used by ardour,
            // the point is to have a number with a lot of dividends
            ppqn: 32,
            // ppqn: 1920,
            time_signature: TimeSig { top: 4, bottom: 4 },
        }
    }
}

impl Tempo {
    pub fn new(sample_rate: SampleHz) -> Self {
        let mut tempo = Self::default();
        tempo.sample_rate = sample_rate;
        tempo
    }

    pub fn handle(&mut self, ts: u128) {
        if self.playing && self.start_time.is_none() {
            self.start_time = Some(ts);
        }
        // Set start_time if just stopped
        if !self.playing && self.start_time.is_some() {
            self.start_time = None;
        }
        if self.playing {
            // Update position
            let millis = (ts - self.start_time.unwrap()) / 1000000;
            self.prev_position = Some(self.current_position);
            self.current_position = Ms(millis as f64).to_ticks(self.bpm, self.ppqn);

            if let Some((start, end)) = self.looping {
                if self.current_position >= end {
                    self.prev_position = Some(self.current_position);
                    self.current_position = start;
                    self.start_time = Some(ts);
                }
            }
        }
    }

    // Utilities functions for convertion to ticks
    pub fn beats(&self, val: i64) -> Ticks {
        Beats(val).to_ticks(self.ppqn)
    }

    pub fn bars(&self, val: i64) -> Ticks {
        Bars(val).to_ticks(self.time_signature, self.ppqn)
    }

    pub fn ticks(&self, val: i64) -> Ticks {
        Ticks(val)
    }
}
