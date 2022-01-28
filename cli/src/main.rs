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
use core::engine::{Clip, ClipNote, Engine};
use core::graph::{Note, SetParam};
use core::module::N_SAMPLES_PER_CHUNK;
use core::modules as m;
use core::worker::Worker;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::BTreeMap;
use std::io::{stdin, stdout, Write};
use std::time::Instant;
use time_calc::{Bars, Beats, Ppqn, Ticks, TimeSig};

fn make_synth(engine: &mut Engine, sample_rate: f32) -> (usize, usize, usize) {
    // Bass synth definition
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
    (synth, pitch, adsr)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the audio worker
    let (worker, tx, rx) = Worker::create(1024);

    // Initialize the audio callback
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
    dbg!(sample_rate);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
    };
    // Play the stream
    stream.play()?;

    // Initialize the audio engine
    let mut engine = Engine::new(sample_rate, rx, tx);

    // Bass synth
    let bass_track = engine.add_track();
    let (bass_synth, bass_pitch, bass_adsr) = make_synth(&mut engine, sample_rate);
    let bass_control = vec![(bass_pitch, 0), (bass_adsr, 0)];
    // Add device to track
    engine.set_track_node(bass_track, [(bass_synth, 0)], bass_control.clone());

    // Lead synth
    let lead_track = engine.add_track();
    let (lead_synth, lead_pitch, lead_adsr) = make_synth(&mut engine, sample_rate);
    let lead_control = vec![(lead_pitch, 0), (lead_adsr, 0)];
    // Add device to track
    engine.set_track_node(lead_track, [(lead_synth, 0)], lead_control.clone());

    dbg!(bass_synth, bass_pitch, bass_adsr);
    dbg!(lead_synth, lead_pitch, lead_adsr);
    let beat = std::time::Duration::from_millis(1000);

    engine.send_note_on(vec![lead_pitch, lead_adsr], 54., 100.);
    std::thread::sleep(beat);
    engine.send_note_on(vec![bass_pitch, bass_adsr], 42., 50.);
    std::thread::sleep(beat);
    engine.send_note_off(vec![lead_pitch, lead_adsr], 54.);
    engine.send_note_on(vec![lead_pitch, lead_adsr], 55., 50.);
    engine.send_note_off(vec![bass_pitch, bass_adsr], 42.);
    engine.send_note_on(vec![bass_pitch, bass_adsr], 40., 150.);
    std::thread::sleep(beat);

    //     engine.send_note_on(bass_control.clone(), 42., 50.);
    //
    //     let mut cur_note = Some(42.);
    //     loop {
    //         print!("Enter note number:\n");
    //         stdout().flush()?;
    //         let mut input = String::new();
    //         stdin().read_line(&mut input)?;
    //         if let Ok(note) = input.trim().parse::<f32>() {
    //             if let Some(prev_note) = cur_note {
    //                 engine.send_note_off(bass_control.clone(), prev_note);
    //             }
    //             engine.send_note_on(bass_control.clone(), note, 50.);
    //             engine.set_param(SetParam {
    //                 ix: attack,
    //                 param_ix: attack,
    //                 val: 150.,
    //                 timestamp: 0,
    //             });
    //             cur_note = Some(note);
    //         } else if input.trim() == "q" {
    //             break;
    //         } else {
    //             if let Some(prev_note) = cur_note {
    //                 engine.send_note_off(bass_control.clone(), prev_note);
    //             }
    //             cur_note = None;
    //         }
    //     }

    // // Create a clip
    // let ts = TimeSig { top: 4, bottom: 4 };
    // let ppqn = 8;
    // let mut clip = Clip::new(Bars(1).to_ticks(ts, ppqn));
    // let note = ClipNote::new(31., Beats(1).to_ticks(ppqn));
    // clip.add_note(note, Ticks(0));
    // let note = ClipNote::new(32., Beats(1).to_ticks(ppqn));
    // clip.add_note(note, Beats(3).to_ticks(ppqn));
    // // Now add clip to track
    // engine.add_clip_to_track(bass_track, clip, Ticks(0));
    // // Set loop region
    // engine.set_loop(Ticks(0), Bars(1).to_ticks(ts, ppqn));
    // // Play
    // engine.play();
    // engine.run();
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
                    // TODO: Request fixes sized buffer length if alsa
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
