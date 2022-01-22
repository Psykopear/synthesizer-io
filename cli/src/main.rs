// Copyright 2017 The Synthesizer IO Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use core::engine::Engine;
use core::module::N_SAMPLES_PER_CHUNK;
use core::modules as m;
use core::worker::Worker;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::{stdin, stdout, Write};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the audio worker
    let (worker, tx, rx) = Worker::create(1024);

    // Initialize the audio callback
    // let host = cpal::default_host();

    let host = cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable");
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

    // Init track
    let bass_track = engine.add_track();
    dbg!(sample_rate);

    // Bass synth definition
    // Note control
    let pitch = engine.create_node(m::NotePitch::new(), [], []);
    // Oscillator
    let saw = engine.create_node(m::Saw::new(sample_rate), [], [(pitch, 0)]);
    // Filter
    let freq = engine.create_node(m::SmoothCtrl::new(440.0f32.log2()), [], []);
    let reso = engine.create_node(m::SmoothCtrl::new(0.2), [], []);
    let filter = engine.create_node(
        m::Biquad::new(sample_rate),
        [(saw, 0)],
        [(freq, 0), (reso, 0)],
    );
    // Envelope
    let attack = engine.create_node(m::SmoothCtrl::new(5.), [], []);
    let decay = engine.create_node(m::SmoothCtrl::new(5.), [], []);
    let sustain = engine.create_node(m::SmoothCtrl::new(4.), [], []);
    let release = engine.create_node(m::SmoothCtrl::new(5.), [], []);
    let adsr = engine.create_node(
        m::Adsr::new(),
        [],
        vec![(attack, 0), (decay, 0), (sustain, 0), (release, 0)],
    );
    // Output
    let bass_synth = engine.create_node(m::Gain::new(), [(filter, 0)], [(adsr, 0)]);
    // Add device to track
    engine.set_track_node(bass_track, [(bass_synth, 0)]);

    engine.send_note_on(vec![pitch, adsr], 42., 50.);
    print!("Enter Enter to stop playing");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;

    // Create a clip
    // let mut clip = Clip::new(Bars(1));
    // let note = Note {
    //     freq: 49.9,
    //     dur: Beats(1).to_ticks(),
    // };
    // clip.add_note(&note, Ticks(0));
    // clip.add_note(&note, Beats(3).to_ticks());

    // Set loop region
    // engine.loop(Bars(0), Bars(1));
    // Play
    // engine.play();
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
    // let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut i = 0;
            while i < data.len() {
                let ts = Instant::now()
                    .saturating_duration_since(start_time)
                    .as_nanos();
                let buf = worker.work(ts)[0].get();
                for j in 0..N_SAMPLES_PER_CHUNK {
                    if data.len() > (i + j * 2 + 1) {
                        let value: T = cpal::Sample::from::<f32>(&buf[j]);
                        data[i + j * 2] = value;
                        data[i + j * 2 + 1] = value;
                    }
                }
                i += N_SAMPLES_PER_CHUNK * 2;
            }
        },
        err_fn,
    )?;
    Ok(stream)
}
