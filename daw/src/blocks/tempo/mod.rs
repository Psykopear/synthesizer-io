use core::time_calc::{TimeSig, Ms};

use druid::{
    widget::{Flex, Label},
    Widget, WidgetExt, LensExt,
};

use crate::{
    state::tempo::Tempo,
    widgets::{SignatureLabel, TapTempo, TempoLabel, Switch},
};

pub fn tempo() -> impl Widget<Tempo> {
    Flex::row()
        .with_child(Label::new("TEMPO: "))
        .with_child(TapTempo::new(|state: &mut Tempo, value: f64| state.bpm = value).padding(4.))
        .with_spacer(4.)
        .with_child(TempoLabel::new().padding(4.).lens(Tempo::bpm))
        .with_spacer(4.)
        .with_child(
            SignatureLabel::new()
                .padding(4.)
                .lens(Tempo::time_signature.map(
                    |t| (t.top, t.bottom),
                    |x, y| {
                        *x = TimeSig {
                            top: y.0,
                            bottom: y.1,
                        }
                    },
                )),
        )
        .with_spacer(4.)
        .with_child(Label::dynamic(|tempo: &Tempo, _env| {
            format!(
                "Bar {:.0} Beat {:.0}",
                tempo.current_bar.0, tempo.current_beat.0
            )
        }))
        .with_flex_spacer(1.)
        .with_child(Switch::new("").lens(Tempo::play))
        .with_spacer(4.)
        .with_child(Label::new("").with_text_size(24.).padding(4.).on_click(
            |_, tempo: &mut Tempo, _| {
                tempo.play.on = false;
                tempo.current_time = Ms(0.);
            },
        ))
        .with_spacer(4.)
        .with_child(Switch::new("雷").lens(Tempo::rec))
        .with_spacer(4.)
        .with_child(Switch::new("ﯩ").lens(Tempo::looping))
        .with_flex_spacer(1.)
        .with_child(Label::new("MONITOR: "))
        .with_child(Label::new(" CPU "))
        .with_spacer(4.)
        .with_child(Label::new(" DISK "))
        .padding((0., 0., 0., 6.))
}
