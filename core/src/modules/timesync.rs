//! A module for monitoring timestamp

use crate::module::{Buffer, Module};
use crate::queue::{Queue, Receiver, Sender, Item};

pub struct TimeSync {
    from_monitor: Sender<u128>,
    tick: u128,
    time: u128,
}

impl TimeSync {
    pub fn new(tick: u128) -> (TimeSync, Receiver<u128>) {
        let (from_monitor, rx) = Queue::new();
        let monitor = TimeSync {
            from_monitor,
            time: 0,
            tick,
        };
        (monitor, rx)
    }
}

impl Module for TimeSync {
    fn n_bufs_out(&self) -> usize {
        1
    }

    fn process(
        &mut self,
        _control_in: &[f32],
        _control_out: &mut [f32],
        _buf_in: &[&Buffer],
        _buf_out: &mut [Buffer],
    ) {
        self.from_monitor.send_item(Item::make_item(self.time));
        self.time += self.tick;
    }
}
