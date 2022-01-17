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

use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::thread;

use cpal::{EventLoop, StreamData, UnknownTypeOutputBuffer};
use druid::widget::{Button, Flex, Widget, WidgetExt};
use druid::{AppLauncher, WindowDesc};
use midir::{MidiInput, MidiInputConnection};
use synth::SynthState;
use synthesizer_io_core::engine::Engine;
use synthesizer_io_core::graph::Node;
use synthesizer_io_core::module::N_SAMPLES_PER_CHUNK;
use synthesizer_io_core::modules;
use synthesizer_io_core::worker::Worker;
use ui::{Patcher, Piano, Scope, JUMPER_MODE, MODULE, WIRE_MODE};

/// Build the main window UI
fn build_ui() -> impl Widget<SynthState> {
    // let button = Label::new("Synthesizer IO");
    let patcher = Patcher::new().on_click(|ctx, data, env| {
        // ctx.submit_command(PATCH.with(delta.clone));
    });
    // patcher.event
    //     ui.add_listener(patcher, move |delta: &mut Vec<Delta>, mut ctx| {
    //         ctx.poke_up(&mut Action::Patch(delta.clone()));
    //     });
    let scope = Scope::new();
    //     ui.add_listener(scope, move |_event: &mut (), mut ctx| {
    //         let mut action = Action::Poll(Default::default());
    //         ctx.poke_up(&mut action);
    //         if let Action::Poll(samples) = action {
    //             ctx.poke(scope, &mut ScopeCommand::Samples(samples));
    //             //println!("polled {} events", _n_msg);
    //         }
    //     });
    let piano = Piano::new();
    //     ui.add_listener(piano, move |event: &mut NoteEvent, mut ctx| {
    //         ctx.poke_up(&mut Action::Note(event.clone()));
    //     });
    let modules = &["sine", "control", "saw", "biquad", "adsr", "gain"];
    let wire_b = Button::new("wire").on_click(|ctx, data: &mut SynthState, env| {
        ctx.submit_command(WIRE_MODE);
    });
    let jumper_b = Button::new("jumper").on_click(|ctx, data, env| {
        ctx.submit_command(JUMPER_MODE);
    });
    let mut buttons = vec![wire_b, jumper_b];
    for &module in modules {
        let button = Button::new(module).on_click(|ctx, data, env| {
            ctx.submit_command(MODULE.with(module.into()));
        });
        buttons.push(button);
    }
    //     let button_row = padded_flex_row(&buttons, ui);
    let mut button_row = Flex::row();
    for button in buttons {
        button_row.add_child(button);
    }
    let mut column = Flex::column();
    let mut mid_row = Flex::row();
    mid_row.add_flex_child(patcher, 3.0);
    mid_row.add_flex_child(scope, 2.0);
    column.add_flex_child(mid_row, 3.0);
    column.add_flex_child(piano, 1.0);
    column
    //     synth_state
    // Label::new("CIAO")
}

fn main() {
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
    let launcher = AppLauncher::with_window(window);
    let _midi_connection = setup_midi(engine); // keep from being dropped
    thread::spawn(move || run_cpal(worker));
    launcher
        .log_to_console()
        .launch(synth_state)
        .expect("launch failed");
}

fn setup_midi(engine: Arc<Mutex<Engine>>) -> Option<MidiInputConnection<()>> {
    let mut midi_in = MidiInput::new("midir input").expect("can't create midi input");
    midi_in.ignore(::midir::Ignore::None);
    let result = midi_in.connect(
        0,
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

fn run_cpal(mut worker: Worker) {
    let event_loop = EventLoop::new();
    let device = cpal::default_output_device().expect("no output device");
    let mut supported_formats_range = device
        .supported_output_formats()
        .expect("error while querying formats");
    let format = supported_formats_range
        .next()
        .expect("no supported format?!")
        .with_max_sample_rate();
    println!("format: {:?}", format);
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id);

    event_loop.run(move |_stream_id, stream_data| {
        match stream_data {
            StreamData::Output {
                buffer: UnknownTypeOutputBuffer::F32(mut buf),
            } => {
                let buf_slice = buf.deref_mut();
                let mut i = 0;
                let mut timestamp = time::precise_time_ns();
                while i < buf_slice.len() {
                    // should let the graph generate stereo
                    let buf = worker.work(timestamp)[0].get();
                    for j in 0..N_SAMPLES_PER_CHUNK {
                        buf_slice[i + j * 2] = buf[j];
                        buf_slice[i + j * 2 + 1] = buf[j];
                    }

                    // TODO: calculate properly, magic value is 64 * 1e9 / 44_100
                    timestamp += 1451247 * (N_SAMPLES_PER_CHUNK as u64) / 64;
                    i += N_SAMPLES_PER_CHUNK * 2;
                }
            }
            _ => panic!("Can't handle output buffer format"),
        }
    });
}
