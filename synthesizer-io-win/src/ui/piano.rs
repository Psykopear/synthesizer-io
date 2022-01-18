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

//! Piano keyboard widget.

use crate::synth::NOTE;
use druid::kurbo::Rect;
use druid::piet::RenderContext;
use druid::widget::Widget;
use druid::BoxConstraints;
use druid::Color;
use druid::Data;
use druid::Env;
use druid::Event;
use druid::EventCtx;
use druid::LifeCycle;
use druid::LifeCycleCtx;
use druid::MouseEvent;
use druid::Size;
use druid::UpdateCtx;
use druid::{LayoutCtx, PaintCtx};
use synthesizer_io_core::engine::NoteEvent;

pub struct Piano {
    start_note: u8,
    end_note: u8,
    pressed: [bool; 128],
    // Note corresponding to mouse press.
    cur_note: Option<u8>,
    dragging: bool,
}

const OCTAVE_WIDTH: i32 = 14;

const NOTE_POS: &[(u8, u8)] = &[
    (0, 0),
    (1, 1),
    (2, 0),
    (3, 1),
    (4, 0),
    (6, 0),
    (7, 1),
    (8, 0),
    (9, 1),
    (10, 0),
    (11, 1),
    (12, 0),
];

const INSET: f32 = 2.0;

impl<T: Data> Widget<T> for Piano {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseMove(event) => {
                if !self.dragging { return };
                let u = event.pos.x / ctx.size().width;
                let v = event.pos.y / ctx.size().height;
                let mut cur_note = None;
                for note in self.start_note..self.end_note {
                    let (u0, v0, u1, v1) = self.note_geom(note);
                    if u >= u0 as f64 && u < u1 as f64 && v >= v0 as f64 && v < v1 as f64 {
                        cur_note = Some(note);
                        break;
                    }
                }
                if cur_note != self.cur_note {
                    if let Some(note) = self.cur_note {
                        self.pressed[note as usize] = false;
                        ctx.submit_command(NOTE.with(NoteEvent {
                            down: false,
                            note,
                            velocity: 0,
                        }));
                        ctx.request_paint();
                    }
                }
                self.handle_mouse(ctx, event);
            }
            Event::MouseDown(event) => {
                self.handle_mouse(ctx, event);
                self.dragging = true;
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);
                if let Some(note) = self.cur_note {
                    self.pressed[note as usize] = false;
                    ctx.submit_command(NOTE.with(NoteEvent {
                        down: false,
                        note,
                        velocity: 0,
                    }));
                    ctx.request_paint();
                }
                self.cur_note = None;
                self.dragging = false;
            }
            _ => (),
        }
    }
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {}
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {}

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let black = ctx.solid_brush(Color::Rgba32(0x080800ff));
        let white = ctx.solid_brush(Color::Rgba32(0xf0f0eaff));
        let active = ctx.solid_brush(Color::Rgba32(0x107010ff));
        // let (x, y) = geom.pos;
        let (x, y) = (0.0, 0.0);

        for note in self.start_note..self.end_note {
            let (u0, v0, u1, v1) = self.note_geom(note);
            let color = if self.pressed[note as usize] {
                &active
            } else {
                if v0 == 0.0 {
                    &black
                } else {
                    &white
                }
            };
            let x0 = x + u0 * ctx.size().width as f32 + INSET;
            let y0 = y + v0 * ctx.size().height as f32 + INSET;
            let x1 = x + u1 * ctx.size().width as f32 - INSET;
            let y1 = y + v1 * ctx.size().height as f32 - INSET;

            ctx.fill(Rect::new(x0 as f64, y0 as f64, x1 as f64, y1 as f64), color);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.max()
    }
}

impl Piano {
    pub fn new() -> Piano {
        Piano {
            start_note: 48,
            end_note: 72,
            pressed: [false; 128],
            cur_note: None,
            dragging: false,
        }
    }

    fn note_pos(&self, note: u8) -> (i32, i32) {
        let octave = note / 12;
        let (x, y) = NOTE_POS[(note % 12) as usize];
        (OCTAVE_WIDTH * (octave as i32) + (x as i32), y as i32)
    }

    // Geometry is in unit square
    fn note_geom(&self, note: u8) -> (f32, f32, f32, f32) {
        let start_x = self.note_pos(self.start_note).0;
        let width = self.note_pos(self.end_note - 1).0 - start_x + 2;
        let width_scale = 1.0 / (width as f32);
        let (x, y) = self.note_pos(note);
        let u = (x - start_x) as f32 * width_scale;
        let v = y as f32 * 0.5;
        (u, 0.5 - v, 2.0 * width_scale + u, 1.0 - v)
    }

    fn handle_mouse(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        ctx.set_active(true);
        let u = event.pos.x / ctx.size().width;
        let v = event.pos.y / ctx.size().height;
        for note in self.start_note..self.end_note {
            let (u0, v0, u1, v1) = self.note_geom(note);
            if u >= u0 as f64 && u < u1 as f64 && v >= v0 as f64 && v < v1 as f64 {
                self.cur_note = Some(note);
                break;
            }
        }
        if let Some(note) = self.cur_note {
            self.pressed[note as usize] = true;
            ctx.submit_command(NOTE.with(NoteEvent {
                down: true,
                note,
                velocity: 100,
            }));
            ctx.request_paint();
        }
    }
}
