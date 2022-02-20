use time_calc::{Ticks, TimeSig, Bpm, Ppqn, SampleHz, Ms};

#[derive(Clone, PartialEq)]
pub struct Transport {
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

impl Default for Transport {
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
            ppqn: 32,
            // ppqn: 1920,
            time_signature: TimeSig { top: 4, bottom: 4 },
        }
    }
}

impl Transport {
    pub fn new(sample_rate: SampleHz) -> Self {
        let mut transport = Self::default();
        transport.sample_rate = sample_rate;
        transport
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
}
