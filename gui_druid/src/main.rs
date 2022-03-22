mod app;
mod blocks;
mod state;
mod widgets;

use druid::{AppLauncher, WindowDesc};
use state::AppState;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = WindowDesc::new(app::App::build_ui())
        .title("ReDAW")
        .window_size((1024., 800.));
    let launcher = AppLauncher::with_window(window).delegate(app::Delegate {});
    let event_sink = launcher.get_external_handle();

    let (mut engine, _stream) = audio::start_engine();
    // engine.set_play();
    audio::example_loop(&mut engine);
    std::thread::spawn(move || {
        let sleep_dur = Duration::from_millis(1);
        loop {
            let response = engine.run_step();
            if let Some(ts) = response {
                event_sink.add_idle_callback(move |data: &mut AppState| {
                    data.tempo.step(ts);
                });
                std::thread::yield_now();
            } else {
                std::thread::sleep(sleep_dur);
            }
        }
    });

    launcher
        .log_to_console()
        .launch(AppState::new())
        .expect("launch failed");
    Ok(())
}
