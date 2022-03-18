use core::{engine::{Engine, note::ClipNote}, worker::Worker, module::N_SAMPLES_PER_CHUNK, modules::{NotePitch, Saw, SmoothCtrl, Biquad, Adsr, Gain}};
use std::time::Instant;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use eframe::{egui::{self, Widget}, epi};

pub struct TemplateApp {
    engine: Engine,
    _stream: Box<dyn StreamTrait>
}

impl Default for TemplateApp {
    fn default() -> Self {
        // Initialize the audio worker
        println!("Init worker");
        let (worker, tx, rx) = Worker::create(1024);

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
        let (mut engine, sender, receiver) = Engine::new(sample_rate, rx, tx);
        engine.init();
        engine.set_play();

        engine.set_loop(engine.tempo.ticks(0), engine.tempo.bars(2));
        // Bass synth
        let bass_track = engine.add_track();
        let (bass_synth, bass_pitch, bass_adsr) = make_synth(&mut engine);
        let bass_control = vec![bass_pitch, bass_adsr];
        // Add device to track
        engine.set_track_node(bass_track, [(bass_synth, 0)], bass_control.clone());
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
        Self {
            engine,
            _stream: Box::new(stream),
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "ReDAW"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let Self { engine, .. } = self;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Bars: {} Beats: {}", engine.tempo.current_bars() as u64, engine.tempo.current_beats() as u64));
            if ui.button("Play").clicked() {
                engine.set_play();
            }
            if ui.button("Pause").clicked() {
                engine.set_pause();
            }
        });

        engine.run_step();
        if engine.tempo.playing {
            ctx.request_repaint();
        }
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

    // let start_time = StreamInstant { secs: 0, nanos: 0 };
    // let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    //
    // let stream = device.build_output_stream(
    //     config,
    //     move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
    //         let ts = _info
    //             .timestamp()
    //             .callback
    //             .duration_since(&start_time)
    //             .unwrap()
    //             .as_nanos();
    //         let mut i = 0;
    //         // let ts = Instant::now().duration_since(start_time).as_nanos();
    //         worker.send_timestamp(ts);
    //         while i < data.len() {
    //             let ts = Instant::now().duration_since(start_time).as_nanos();
    //             let buf = worker.work(ts)[0].get();
    //             for j in 0..N_SAMPLES_PER_CHUNK {
    //                 let value: T = cpal::Sample::from::<f32>(&buf[j]);
    //                 data[i + j * 2] = value;
    //                 data[i + j * 2 + 1] = value;
    //             }
    //             i += N_SAMPLES_PER_CHUNK * 2;
    //         }
    //     },
    //     err_fn,
    // )?;
    // Ok(stream)
    let start_time = Instant::now();
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut i = 0;
            let ts = Instant::now().duration_since(start_time).as_nanos();
            worker.send_timestamp(ts);
            while i < data.len() {
                let ts = Instant::now().duration_since(start_time).as_nanos();
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

// A function to build a basic synth and return its controlling nodes
fn make_synth(engine: &mut Engine) -> (usize, usize, usize) {
    let sample_rate = engine.tempo.sample_rate as f32;
    // Synth definition
    // Note control
    let pitch = engine.create_node(NotePitch::new(), [], []);
    // Oscillator
    let saw = engine.create_node(Saw::new(sample_rate), [], [(pitch, 0)]);
    // Filter
    let freq = engine.create_node(SmoothCtrl::new(440.0f32.log2()), [], []);
    let reso = engine.create_node(SmoothCtrl::new(0.3), [], []);
    let filter = engine.create_node(
        Biquad::new(sample_rate),
        [(saw, 0)],
        [(freq, 0), (reso, 0)],
    );
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
    (synth, pitch, adsr)
}
