use core::engine::{clip::Clip, note::ClipNote, Engine};
use core::module::N_SAMPLES_PER_CHUNK;
use core::modules as m;
use core::worker::Worker;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::{Instant, Duration};
use time_calc::{Bars, Beats, Ticks, TimeSig};

/// A function to build a basic synth and return its controlling nodes
fn make_synth(engine: &mut Engine) -> (usize, usize, usize) {
    let sample_rate = engine.transport.sample_rate as f32;
    // Synth definition
    // Note control
    let pitch = engine.create_node(m::NotePitch::new(), [], []);
    // Oscillator
    let saw = engine.create_node(m::Saw::new(sample_rate), [], [(pitch, 0)]);
    // Filter
    let freq = engine.create_node(m::SmoothCtrl::new(440.0f32.log2()), [], []);
    let reso = engine.create_node(m::SmoothCtrl::new(0.3), [], []);
    let filter = engine.create_node(
        m::Biquad::new(sample_rate),
        [(saw, 0)],
        [(freq, 0), (reso, 0)],
    );
    // Envelope
    let attack = engine.create_node(m::SmoothCtrl::new(5.), [], []);
    let decay = engine.create_node(m::SmoothCtrl::new(5.), [], []);
    let sustain = engine.create_node(m::SmoothCtrl::new(4.), [], []);
    let release = engine.create_node(m::SmoothCtrl::new(10.), [], []);
    let adsr = engine.create_node(
        m::Adsr::new(),
        [],
        vec![(attack, 0), (decay, 0), (sustain, 0), (release, 0)],
    );
    // Output
    let synth = engine.create_node(m::Gain::new(), [(filter, 0)], [(adsr, 0)]);
    // Return ids to control synth and its pitch and envelope
    (synth, pitch, adsr)
}

fn main() {
    // Initialize the audio worker
    println!("Init worker");
    let (worker, tx, rx, control_rx) = Worker::create(1024);

    // Initialize the audio callback
    println!("Init host");
    let host = cpal::available_hosts()
        .into_iter()
        .find(|id| *id == cpal::HostId::Jack)
        .map_or_else(
            || cpal::default_host(),
            |id| cpal::host_from_id(id).unwrap(),
        );

    println!("Init device");
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;
    dbg!(sample_rate);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
    };
    // Play the stream
    stream.play().unwrap();

    // Initialize the audio engine
    let mut engine = Engine::new(sample_rate, rx, tx);
    engine.init();
    engine.play();
    engine.set_loop(
        Ticks(0),
        Bars(1).to_ticks(engine.transport.time_signature, engine.transport.ppqn),
    );
    // Bass synth
    let bass_track = engine.add_track();
    let (bass_synth, bass_pitch, bass_adsr) = make_synth(&mut engine);
    let bass_control = vec![bass_pitch, bass_adsr];
    // Add device to track
    engine.set_track_node(bass_track, [(bass_synth, 0)], bass_control.clone());
    // Create an empty clip
    let clip = engine.add_clip_to_track(bass_track, Ticks(0));
    // Add some notes to the clip
    engine.add_note(
        bass_track,
        clip,
        ClipNote {
            dur: Beats(2).to_ticks(engine.transport.ppqn),
            midi: 42.,
            vel: 100,
        },
        Ticks(0),
    );
    engine.add_note(
        bass_track,
        clip,
        ClipNote {
            dur: Beats(1).to_ticks(engine.transport.ppqn),
            midi: 41.,
            vel: 100,
        },
        Beats(2).to_ticks(engine.transport.ppqn),
    );

    loop {
        for ts in control_rx.recv_items() {
            engine.run_step(*ts);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
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

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut i = 0;
            while i < data.len() {
                let ts = Instant::now()
                    .saturating_duration_since(start_time)
                    .as_nanos();
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
