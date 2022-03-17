mod blocks;
mod state;
mod widgets;

use blocks::tempo;
use core::{engine::Engine, module::N_SAMPLES_PER_CHUNK, worker::Worker};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use druid::{AppLauncher, Data, Lens, Widget, WidgetExt, WindowDesc};
use state::tempo::Tempo;
use std::time::Instant;

#[derive(Clone, Data, Lens)]
struct AppState {
    tempo: Tempo,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tempo: Default::default(),
        }
    }
}

fn build_ui() -> impl Widget<AppState> {
    tempo().lens(AppState::tempo)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the audio worker
    let (worker, tx, rx, control_rx) = Worker::create(65536);

    // Init host, jack if possible
    let host = cpal::available_hosts()
        .into_iter()
        .find(|id| *id == cpal::HostId::Jack)
        .map_or_else(
            || cpal::default_host(),
            |id| cpal::host_from_id(id).unwrap(),
        );
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;

    // Initialize the audio engine
    let mut engine = Engine::new(sample_rate, rx, tx, control_rx);
    engine.init();
    std::thread::spawn(move || engine.run());
    let window = WindowDesc::new(build_ui()).title("Synthesizer IO");
    let launcher = AppLauncher::with_window(window);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
    };
    // Play the stream
    stream.play()?;

    launcher
        .log_to_console()
        .launch(AppState::new())
        .expect("launch failed");
    Ok(())
}

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut worker: Worker,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::Sample,
{
    let start_time = Instant::now();
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let mut check = 0;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            if check == 0 {
                let ts = Instant::now().duration_since(start_time).as_nanos();
                worker.send_timestamp(ts);
            }
            check = (check + 1) % 4;
            let mut i = 0;
            while i < data.len() {
                let ts = Instant::now().duration_since(start_time).as_nanos();
                worker.send_timestamp(ts);
                let buf = worker.work(ts)[0].get();
                for j in 0..N_SAMPLES_PER_CHUNK {
                    let value: T = cpal::Sample::from::<f32>(&buf[j]);
                    data[i + j * 2] = value;
                    data[i + j * 2 + 1] = value;
                }
                i += N_SAMPLES_PER_CHUNK * 2;
            }
        },
        err_fn,
    )?;
    Ok(stream)
}
