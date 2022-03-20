mod app;
mod blocks;
mod state;
mod widgets;

use druid::{AppLauncher, Data, Lens, WindowDesc};
use state::tempo::Tempo;
use std::time::Duration;

#[derive(Clone, Data, Lens)]
pub struct AppState {
    tempo: Tempo,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tempo: Default::default(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut engine, _stream) = audio::start_engine();
    engine.set_play();
    audio::example_loop(&mut engine);
    std::thread::spawn(move || {
        let sleep_dur = Duration::from_millis(1);
        loop {
            engine.run_step();
            std::thread::sleep(sleep_dur);
        }
    });

    let window = WindowDesc::new(app::App::build_ui())
        .title("ReDAW")
        .window_size((1024., 800.));
    let launcher = AppLauncher::with_window(window).delegate(app::Delegate {});

    launcher
        .log_to_console()
        .launch(AppState::new())
        .expect("launch failed");
    Ok(())
}
