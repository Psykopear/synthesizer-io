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
    stream: cpal::Stream,
}

impl Redaw {
    pub fn new(
        stream: cpal::Stream,
        rx: Receiver<core::graph::Message>,
        _tx: Sender<core::graph::Message>,
    ) -> Self {
        let mut tempo = Tempo::default();
        tempo.playing = true;
        Self {
            tempo,
            rx,
            _tx,
            stream,
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
        let (mut engine, stream) = audio::start_engine();
        audio::example_loop(&mut engine);
        std::thread::spawn(move || loop {
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
