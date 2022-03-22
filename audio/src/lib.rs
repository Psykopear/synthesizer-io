use assert_no_alloc::*;

#[cfg(debug_assertions)]
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

use core::{
    engine::{Engine, note::ClipNote},
    module::N_SAMPLES_PER_CHUNK,
    modules::{Adsr, Biquad, Gain, NotePitch, Saw, SmoothCtrl},
    worker::Worker,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut worker: Worker,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::Sample,
{
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let sample_rate = config.sample_rate;
    let ts_step =
         (1_000_000_000 * N_SAMPLES_PER_CHUNK as u128) / sample_rate.0 as u128;

    // TODO: We can either use the timestamp from the audio callback,
    //       or keep track of nanoseconds passed ourselves, knowing the
    //       sample rate and the data length. Make an informed decision.
    let mut ts = 0;
    // let mut start_time = None;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
            assert_no_alloc(|| {
                // if start_time.is_none() {
                //     start_time = Some(info.timestamp().callback);
                // }
                // let mut ts = info
                //     .timestamp()
                //     .callback
                //     .duration_since(&start_time.unwrap())
                //     .unwrap()
                //     .as_nanos();
                // println!("Pre: {}", ts);
                let mut i = 0;
                worker.send_ts(ts);
                while i < data.len() {
                    let buf = worker.work(ts)[0].get();
                    // TODO: This won't work if the audio buffer size is smaller than
                    // N_SAMPLES_PER_CHUNK
                    for j in 0..N_SAMPLES_PER_CHUNK {
                        let value: T = cpal::Sample::from::<f32>(&buf[j]);
                        data[i + j * 2] = value;
                        data[i + j * 2 + 1] = value;
                    }
                    i += N_SAMPLES_PER_CHUNK * 2;
                    ts += ts_step;
                }
            });
        },
        err_fn,
    )?;
    Ok(stream)
}

pub fn start_engine() -> (Engine, cpal::Stream) {
    // Initialize the audio worker
    println!("Init worker");
    let (worker, tx, rx, ts_receiver) = Worker::create(1024);

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

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
    };
    // Play the stream
    stream.play().unwrap();

    // Initialize the audio engine
    let mut engine = Engine::new(sample_rate, rx, tx, ts_receiver);
    engine.init();

    (engine, stream)
}

pub fn example_loop(engine: &mut Engine) {
    engine.set_loop(engine.tempo.ticks(0), engine.tempo.bars(2));
    // Bass synth
    let bass_track = engine.add_track();
    let ([bass_synth], bass_ctrl) = make_synth(engine);
    // Add device to track
    engine.set_track_node(bass_track, [(bass_synth, 0)], bass_ctrl.to_vec());
    // Create an empty clip
    let clip = engine.add_clip_to_track(bass_track, engine.tempo.ticks(0));
    // Add some notes to the clip
    let note = ClipNote {
        dur: engine.tempo.beats(2),
        midi: 31.,
        vel: 100,
    };
    engine.add_note(bass_track, clip, note, engine.tempo.ticks(0));
    let note = ClipNote {
        dur: engine.tempo.beats(1),
        midi: 30.,
        vel: 50,
    };
    engine.add_note(bass_track, clip, note, engine.tempo.beats(1));
    let note = ClipNote {
        dur: engine.tempo.beats(2),
        midi: 33.,
        vel: 100,
    };
    engine.add_note(bass_track, clip, note, engine.tempo.bars(1));
}

/// A function to build a basic synth and return its controlling nodes
pub fn make_synth(engine: &mut Engine) -> ([usize; 1], [usize; 2]) {
    let sample_rate = engine.tempo.sample_rate as f32;
    // Synth definition
    // Note control
    let pitch = engine.create_node(NotePitch::new(), [], []);
    // Oscillator
    let saw = engine.create_node(Saw::new(sample_rate), [], [(pitch, 0)]);
    // Filter
    let freq = engine.create_node(SmoothCtrl::new(440.0f32.log2()), [], []);
    let reso = engine.create_node(SmoothCtrl::new(0.3), [], []);
    let filter = engine.create_node(Biquad::new(sample_rate), [(saw, 0)], [(freq, 0), (reso, 0)]);
    // Envelope
    let attack = engine.create_node(SmoothCtrl::new(5.), [], []);
    let decay = engine.create_node(SmoothCtrl::new(5.), [], []);
    let sustain = engine.create_node(SmoothCtrl::new(4.), [], []);
    let release = engine.create_node(SmoothCtrl::new(10.), [], []);
    let adsr = engine.create_node(
        Adsr::new(),
        [],
        vec![(attack, 0), (decay, 0), (sustain, 0), (release, 0)],
    );
    // Output
    let synth = engine.create_node(Gain::new(), [(filter, 0)], [(adsr, 0)]);
    // Return ids to control synth and its pitch and envelope
    ([synth], [pitch, adsr])
}
