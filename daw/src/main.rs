mod engine;

use engine::Engine;

use core::module::N_SAMPLES_PER_CHUNK;
use core::worker::Worker;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use druid::{widget::Label, AppDelegate, AppLauncher, Data, Widget, WindowDesc};
use midir::{MidiInput, MidiInputConnection};

#[derive(Clone, Data)]
struct AppState {
    engine: Engine,
}

struct Delegate {}

impl AppDelegate<AppState> for Delegate {}

fn build_ui() -> impl Widget<AppState> {
    Label::new("DAW")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the audio worker
    let (worker, tx, rx, control_rx) = Worker::create(1024);

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
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
    };
    // Play the stream
    stream.play()?;

    // Initialize the audio engine
    let mut engine = Engine::new(sample_rate, rx, tx, control_rx);
    engine.init();
    let appstate = AppState {
        engine
    };
    let window = WindowDesc::new(build_ui()).title("Synthesizer IO");
    let launcher = AppLauncher::with_window(window).delegate(Delegate {});
    launcher
        .log_to_console()
        .launch(appstate)
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
    // let sample_rate = config.sample_rate.0;
    let start_time = Instant::now();
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let ts = Instant::now().duration_since(start_time).as_nanos();
            worker.send_timestamp(ts);

            let mut i = 0;
            while i < data.len() {
                let ts = Instant::now()
                    .saturating_duration_since(start_time)
                    .as_nanos();
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
