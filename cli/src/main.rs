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

use core::graph::{Message, Node, Note, SetParam};
use core::module::N_SAMPLES_PER_CHUNK;
use core::modules;
use core::queue::Sender;
use core::worker::Worker;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midir::{MidiInput, MidiInputConnection};
use std::io::Write;

struct Midi {
    tx: Sender<Message>,
    cur_note: Option<u8>,
}

impl Midi {
    fn new(tx: Sender<Message>) -> Midi {
        Midi { tx, cur_note: None }
    }

    fn send(&self, msg: Message) {
        self.tx.send(msg);
    }

    fn set_ctrl_const(&mut self, value: u8, lo: f32, hi: f32, ix: usize, ts: u64) {
        let val = lo + value as f32 * (1.0 / 127.0) * (hi - lo);
        let param = SetParam {
            ix,
            param_ix: 0,
            val,
            timestamp: ts,
        };
        self.send(Message::SetParam(param));
    }

    fn send_note(&mut self, ixs: Vec<usize>, midi_num: f32, velocity: f32, on: bool, ts: u64) {
        let note = Note {
            ixs: ixs.into_boxed_slice(),
            midi_num,
            velocity,
            on,
            timestamp: ts,
        };
        self.send(Message::Note(note));
    }

    fn dispatch_midi(&mut self, data: &[u8], ts: u64) {
        let mut i = 0;
        while i < data.len() {
            if data[i] == 0xb0 {
                let controller = data[i + 1];
                let value = data[i + 2];
                match controller {
                    1 => self.set_ctrl_const(value, 0.0, 22_000f32.log2(), 3, ts),
                    2 => self.set_ctrl_const(value, 0.0, 0.995, 4, ts),
                    3 => self.set_ctrl_const(value, 0.0, 22_000f32.log2(), 5, ts),
                    5 => self.set_ctrl_const(value, 0.0, 10.0, 11, ts),
                    6 => self.set_ctrl_const(value, 0.0, 10.0, 12, ts),
                    7 => self.set_ctrl_const(value, 0.0, 6.0, 13, ts),
                    8 => self.set_ctrl_const(value, 0.0, 10.0, 14, ts),
                    _ => println!("don't have handler for controller {}", controller),
                }
                i += 3;
            } else if data[i] == 0x90 || data[i] == 0x80 {
                let midi_num = data[i + 1];
                let velocity = data[i + 2];
                let on = data[i] == 0x90 && velocity > 0;
                if on || self.cur_note == Some(midi_num) {
                    self.send_note(vec![5, 7], midi_num as f32, velocity as f32, on, ts);
                    self.cur_note = if on { Some(midi_num) } else { None }
                }
                i += 3;
            } else {
                break;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut worker, tx, _rx) = Worker::create(1024);

    let module = Box::new(modules::Saw::new(44_100.0));
    worker.handle_node(Node::create(module, 1, [], [(5, 0)]));
    let module = Box::new(modules::SmoothCtrl::new(880.0f32.log2()));
    worker.handle_node(Node::create(module, 3, [], []));
    let module = Box::new(modules::SmoothCtrl::new(0.5));
    worker.handle_node(Node::create(module, 4, [], []));
    let module = Box::new(modules::NotePitch::new());
    worker.handle_node(Node::create(module, 5, [], []));
    let module = Box::new(modules::Biquad::new(44_100.0));
    worker.handle_node(Node::create(module, 6, [(1, 0)], [(3, 0), (4, 0)]));
    let module = Box::new(modules::Adsr::new());
    worker.handle_node(Node::create(module, 7, [], vec![(11, 0), (12, 0), (13, 0), (14, 0)],));
    let module = Box::new(modules::Gain::new());
    worker.handle_node(Node::create(module, 0, [(6, 0)], [(7, 0)]));

    let module = Box::new(modules::SmoothCtrl::new(5.0));
    worker.handle_node(Node::create(module, 11, [], []));
    let module = Box::new(modules::SmoothCtrl::new(5.0));
    worker.handle_node(Node::create(module, 12, [], []));
    let module = Box::new(modules::SmoothCtrl::new(4.0));
    worker.handle_node(Node::create(module, 13, [], []));
    let module = Box::new(modules::SmoothCtrl::new(5.0));
    worker.handle_node(Node::create(module, 14, [], []));

    let _midi_connection = setup_midi(tx); // keep from being dropped
    let stream = run_cpal(worker);
    stream.play()?;
    print!("Press Enter to stop the synth...");
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    Ok(())
}

fn setup_midi(tx: Sender<Message>) -> Option<MidiInputConnection<()>> {
    let mut midi = Midi::new(tx);
    let mut midi_in = MidiInput::new("midir input").expect("can't create midi input");
    midi_in.ignore(midir::Ignore::None);
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return None,
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            in_ports
                .get(input.trim().parse::<usize>().unwrap())
                .ok_or("invalid input port selected")
                .unwrap()
        }
    };
    let result = midi_in.connect(
        in_port,
        "in",
        move |ts, data, _| {
            midi.dispatch_midi(data, ts);
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
