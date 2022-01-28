use core::module::N_SAMPLES_PER_CHUNK;
use core::{engine::Engine, worker::Worker};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use druid::{widget::Label, AppDelegate, AppLauncher, Data, Widget, WindowDesc};
use midir::{MidiInput, MidiInputConnection};

#[derive(Clone, Data)]
struct AppState {
    engine: Engine
}

struct Delegate {}

impl AppDelegate<AppState> for Delegate {}

fn build_ui() -> impl Widget<AppState> {
    Label::new("DAW")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the audio worker
    let (worker, tx, rx) = Worker::create(1024);
    // Initialize the audio callback
    let host = cpal::default_host();
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
    let mut engine = Engine::new(sample_rate, rx, tx);
    engine.init_monosynth();
    let engine = Arc::new(Mutex::new(engine));
    let appstate = AppState {
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
    // let sample_rate = config.sample_rate.0 as f32;
    // let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut i = 0;
            let mut timestamp = Instant::now().elapsed().as_millis() as u64;
            while i < data.len() {
                // should let the graph generate stereo
                let buf = worker.work(timestamp)[0].get();
                for j in 0..N_SAMPLES_PER_CHUNK {
                    // TODO: Use fixed sized buffer and avoid this check
                    if data.len() > (i + j * 2 + 1) {
                        let value: T = cpal::Sample::from::<f32>(&buf[j]);
                        data[i + j * 2] = value;
                        data[i + j * 2 + 1] = value;
                    }
                }

                // TODO: calculate properly, magic value is 64 * 1e9 / 44_100
                timestamp += 1451247 * (N_SAMPLES_PER_CHUNK as u64) / 64;
                i += N_SAMPLES_PER_CHUNK * 2;
            }
        },
        err_fn,
    )?;
    Ok(stream)
}
