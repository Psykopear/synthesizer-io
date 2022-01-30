use time_calc::{Ticks, TimeSig};

pub fn ticks(t1: &Ticks, t2: &Ticks) -> bool {
    t1.0 == t2.0
}

pub fn looping(l1: &Option<(Ticks, Ticks)>, l2: &Option<(Ticks, Ticks)>) -> bool {
    if l1.is_none() && l2.is_none() {
        true
    } else if l1.is_none() || l2.is_none() {
        false
    } else {
        let l1 = l1.unwrap();
        let l2 = l2.unwrap();
        ticks(&l1.0, &l2.0) && ticks(&l1.1, &l2.1)
    }
}

pub fn timesig(t1: &TimeSig, t2: &TimeSig) -> bool {
    t1.top == t2.top && t1.bottom == t2.bottom
}

pub fn maybe_ticks(t1: &Option<Ticks>, t2: &Option<Ticks>) -> bool {
    if t1.is_none() && t2.is_none() {
        true
    } else if t1.is_none() || t2.is_none() {
        false
    } else {
        ticks(&t1.unwrap(), &t2.unwrap())
    }
}
