use core::time_calc::Ms;

use druid::{
    widget::{Flex, Label},
    Env, LensExt, Widget, WidgetExt,
};

use crate::{
    state::tempo::Tempo,
    widgets::{SignatureLabel, Switch, TapTempo, TempoLabel},
};

fn tempo_section() -> impl Widget<Tempo> {
    Flex::row()
        // Left section: Tempo
        .with_child(Label::new("TEMPO: "))
        // BPM
        .with_child(TempoLabel::new().padding(4.).lens(Tempo::bpm))
        .with_spacer(4.)
        // Button to set tempo tapping on it
        .with_child(TapTempo::new(|state: &mut Tempo, value: f64| state.bpm = value).padding(4.))
        .with_spacer(4.)
        // Signature label
        .with_child(
            SignatureLabel::new()
                .padding(4.)
                .lens(Tempo::time_signature.map(
                    |t| (t.top, t.bottom),
                    |x, y| {
                        x.top = y.0;
                        x.bottom = y.1;
                    },
                )),
        )
        .with_spacer(4.)
        // Bars/Beats counter
        .with_child(Label::dynamic(|tempo: &Tempo, _env| {
            format!(
                "Bar {:.0} Beat {:.0}",
                tempo.current_bar.0,
                tempo.current_beat.0 % tempo.time_signature.top as i64
            )
        }))
}

fn transport_section() -> impl Widget<Tempo> {
    // Center section: Transport control
    Flex::row()
        // Play button
        .with_child(Switch::new("Play").lens(Tempo::play))
        .with_spacer(4.)
        // Stop button
        .with_child(Label::new("Stop").on_click(
            |_, tempo: &mut Tempo, _| {
                tempo.play.on = false;
                tempo.current_time = Ms(0.);
            },
        ))
        .with_spacer(4.)
        // Record button
        .with_child(Switch::new("Rec").lens(Tempo::rec))
        .with_spacer(4.)
        // Loop button
        .with_child(Switch::new("Loop").lens(Tempo::looping))
}

fn monitor_section() -> impl Widget<Tempo> {
    // Right section: Monitors
    Flex::row()
        .with_child(Label::new("MONITOR: "))
        .with_child(Label::new(" CPU "))
        .with_spacer(4.)
        .with_child(Label::new(" DISK "))
}

pub fn tempo() -> impl Widget<Tempo> {
    Flex::row()
        .with_child(tempo_section())
        .with_flex_spacer(1.)
        .with_child(transport_section())
        .with_flex_spacer(1.)
        .with_child(monitor_section())
        .padding((0., 0., 0., 6.))
}
