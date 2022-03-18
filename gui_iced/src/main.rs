use assert_no_alloc::*;
use cpal::StreamInstant;

#[cfg(debug_assertions)]
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

mod state;
mod widgets;

use core::engine::note::ClipNote;
use core::engine::tempo::Tempo;
use core::engine::Engine;
use core::module::N_SAMPLES_PER_CHUNK;
use core::modules as m;
use core::queue::{Receiver, Sender};
use core::worker::Worker;
use std::ops::DerefMut;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use iced::pure::{button, column, container, row, text, Application, Element};
use iced::{time, window, Alignment, Command, Length, Settings, Subscription};
use widgets::Timeline;

pub struct Redaw {
    tempo: Tempo,
    rx: Receiver<core::graph::Message>,
    _tx: Sender<core::graph::Message>,
    // Reference used to keep the audio thread alive
    _stream: Box<dyn StreamTrait>,
}

impl Redaw {
    pub fn new(
        stream: Box<dyn StreamTrait>,
        rx: Receiver<core::graph::Message>,
        _tx: Sender<core::graph::Message>,
    ) -> Self {
        let mut tempo = Tempo::default();
        tempo.playing = true;
        Self {
            tempo,
            rx,
            _tx,
            _stream: stream,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Play,
    Pause,
}

impl Application for Redaw {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
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
        std::thread::spawn(move || loop {
            // println!("Engine step");
            engine.run_step();
            std::thread::sleep(Duration::from_millis(2));
        });

        (
            Redaw::new(Box::new(stream), receiver, sender),
            Command::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.tempo.playing {
            // Tick every 16ms (~60fps)
            time::every_skip(Duration::from_millis(33)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn title(&self) -> String {
        "ReDAW".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                if let Some(ts) = self
                    .rx
                    .recv_items()
                    .filter_map(|m| {
                        if let core::graph::Message::Timestamp(t) = *m {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .last()
                {
                    // println!("UI TICK");
                    self.tempo.step(ts);
                };
            }
            Message::Play => {
                self.tempo.playing = true;
            }
            Message::Pause => {
                self.tempo.playing = false;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let tempo = &self.tempo;
        let ts = tempo.time_signature;

        // Top bar
        let bpm = text(format!("{}", tempo.bpm));
        let signature = text(format!("{}/{}", ts.top, ts.bottom));
        let current = text(format!(
            "Bars: {} Beats: {}",
            tempo.current_bars() as u64,
            tempo.current_beats() as u64,
        ));
        let play = button("Play").on_press(Message::Play);
        let pause = button("Pause").on_press(Message::Pause);
        let tempo_row = row()
            .width(Length::Fill)
            .align_items(Alignment::Fill)
            .push(text("TEMPO: "))
            .spacing(10)
            .push(bpm)
            .push(signature)
            .push(current);
        let controls = row().push(play).push(pause);
        let top_bar = row().push(tempo_row).push(controls);

        // Timeline
        let timeline = container(Timeline::new());

        // Device
        let device = container(text("Device"));

        let content = column()
            .padding(20)
            .spacing(20)
            .push(top_bar)
            .push(timeline)
            .push(device);

        container(content).into()
    }
}

pub fn main() -> iced::Result {
    Redaw::run(Settings {
        window: window::Settings {
            size: (1024, 800),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut worker: Worker,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::Sample,
{
    let mut start_time = None;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let mut ts_item = core::queue::Item::make_item(core::graph::Message::Timestamp(0));

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
            assert_no_alloc(|| {
                if start_time.is_none() {
                    start_time = Some(info.timestamp().callback);
                }
                let mut i = 0;
                let mut ts = info
                    .timestamp()
                    .playback
                    .duration_since(&start_time.unwrap())
                    .unwrap()
                    .as_micros();
                if let core::graph::Message::Timestamp(mut _ts) = *ts_item.deref_mut() {
                    _ts = ts;
                }

                while i < data.len() {
                    // worker.send_timestamp(ts_item);
                    let buf = worker.work(ts)[0].get();
                    for j in 0..N_SAMPLES_PER_CHUNK {
                        let value: T = cpal::Sample::from::<f32>(&buf[j]);
                        data[i + j * 2] = value;
                        data[i + j * 2 + 1] = value;
                    }
                    i += N_SAMPLES_PER_CHUNK * 2;
                    ts += 1451247 * (N_SAMPLES_PER_CHUNK as u128) / 64;
                }
            });
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
