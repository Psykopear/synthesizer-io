// Copyright 2018 The Synthesizer IO Authors.
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

//! Windows GUI music synthesizer app.

mod grid;
mod synth;
mod ui;

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use druid::widget::{Button, Flex, Label, Widget};
use druid::{AppDelegate, AppLauncher, Command, DelegateCtx, Env, Handled, Target, WindowDesc};
use midir::{MidiInput, MidiInputConnection};
use synth::{SynthState, NOTE, PATCH, POLL};
use synthesizer_io_core::engine::Engine;
use synthesizer_io_core::graph::Node;
use synthesizer_io_core::module::N_SAMPLES_PER_CHUNK;
use synthesizer_io_core::modules;
use synthesizer_io_core::worker::Worker;
use ui::{Patcher, Piano, Scope, JUMPER_MODE, MODULE, SAMPLES, WIRE_MODE};

struct Delegate {}

impl AppDelegate<SynthState> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut SynthState,
        _env: &Env,
    ) -> Handled {
        if let Some(note_event) = cmd.get(NOTE) {
            let mut engine = data.engine.lock().unwrap();
            engine.dispatch_note_event(note_event);
            return Handled::Yes;
        }
        if let Some(delta) = cmd.get(PATCH) {
            data.apply_patch_delta(delta);
            return Handled::Yes;
        }
        if cmd.is(POLL) {
            let mut engine = data.engine.lock().unwrap();
            let _n_msg = engine.poll_rx();
            ctx.submit_command(SAMPLES.with(engine.poll_monitor()));
            return Handled::Yes;
        }
        Handled::No
    }
}

/// Build the main window UI
fn build_ui() -> impl Widget<SynthState> {
    let button = Label::new("Synthesizer IO");
    let patcher = Patcher::new();
    let scope = Scope::new();
    let piano = Piano::new();

    let modules = &["sine", "control", "saw", "biquad", "adsr", "gain"];

    let wire_b = Button::new("wire").on_click(|ctx, _data: &mut SynthState, _env| {
        ctx.submit_command(WIRE_MODE);
    });
    let jumper_b = Button::new("jumper").on_click(|ctx, _data, _env| {
        ctx.submit_command(JUMPER_MODE);
    });

    let mut button_row = Flex::row();
    button_row.add_child(wire_b);
    button_row.add_child(jumper_b);

    for &module in modules {
        button_row.add_child(Button::new(module).on_click(|ctx, _data, _env| {
            ctx.submit_command(MODULE.with(module.into()));
        }));
    }

    Flex::column()
        .with_child(button)
        .with_flex_child(
            Flex::row()
                .with_flex_child(patcher, 3.0)
                .with_flex_child(scope, 2.0),
            3.0,
        )
        .with_child(button_row)
        .with_flex_child(piano, 1.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut worker, tx, rx) = Worker::create(1024);
    // TODO: get sample rate from cpal
    let mut engine = Engine::new(48_000.0, rx, tx);
    engine.init_monosynth();

    let engine = Arc::new(Mutex::new(engine));

    let synth_state = SynthState::new(engine.clone());

    // Set up working graph; will probably be replaced by the engine before
    // the first audio callback runs.
    let module = Box::new(modules::Sum::new());
    worker.handle_node(Node::create(module, 0, [], []));

    let window = WindowDesc::new(build_ui()).title("Synthesizer IO");
    let launcher = AppLauncher::with_window(window).delegate(Delegate {});
    let _midi_connection = setup_midi(engine); // keep from being dropped
    let stream = run_cpal(worker);
    stream.play()?;
    launcher
        .log_to_console()
        .launch(synth_state)
        .expect("launch failed");
    Ok(())
}

fn setup_midi(engine: Arc<Mutex<Engine>>) -> Option<MidiInputConnection<()>> {
    let mut midi_in = MidiInput::new("midir input").expect("can't create midi input");
    midi_in.ignore(midir::Ignore::None);
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return None,
        _ => {
            println!(
                "Choosing the first available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
    };
    let result = midi_in.connect(
        in_port,
        "in",
        move |ts, data, _| {
            println!("{}, {:?}", ts, data);
            let mut engine = engine.lock().unwrap();
            engine.dispatch_midi(data, time::precise_time_ns());
        },
        (),
    );
    if let Err(ref e) = result {
        println!("error connecting to midi: {:?}", e);
    }
    result.ok()
}

fn run_cpal(worker: Worker) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), worker).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), worker).unwrap(),
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
    // let sample_rate = config.sample_rate.0 as f32;
    // let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut i = 0;
            let mut timestamp = time::precise_time_ns();
            while i < data.len() {
                // should let the graph generate stereo
                let buf = worker.work(timestamp)[0].get();
                for j in 0..N_SAMPLES_PER_CHUNK {
                    // TODO: This check wasn't needed in the original version.
                    // data.len() can change, and is not necessarily a multiple
                    // of N_SAMPLES_PER_CHUNK * 2, so at some point I have a chunk
                    // of data and not enough space left inside `data`.
                    // What should I do there? For now I leave the rest of the
                    // buffer as it is, but we lose at most one chunk of data.
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
