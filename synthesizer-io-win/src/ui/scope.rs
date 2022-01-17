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

//! Widget for oscilloscope display.
use druid::{
    kurbo::Rect,
    piet::{ImageFormat, InterpolationMode, RenderContext},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Selector, Size, UpdateCtx, Widget,
};

use synthesize_scope as s;

pub struct Scope {
    // I might want to call the data structure ScopeBuf or some such,
    // too many name collisions :/
    s: s::Scope,
}

pub const START: Selector = Selector::new("synthesizer-io.scope.start");
pub const SAMPLES: Selector<Vec<f32>> = Selector::new("synthesizer-io.scope.samples");

impl<T: Data> Widget<T> for Scope {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) => {
                if let Some(_) = cmd.get(START) {
                    ctx.request_anim_frame();
                }
                if let Some(samples) = cmd.get(SAMPLES) {
                    self.s.provide_samples(&samples);
                }
            }
            Event::AnimFrame(interval) => {
                ctx.request_paint();
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {}

    // fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        // let rc = &mut *paint_ctx.render_ctx;
        let w = 640;
        let h = 480;
        let data = self.s.as_rgba();
        let b = ctx
            .make_image(w, h, &data, ImageFormat::RgbaPremul)
            .unwrap();
        let height = ctx.size().height.min(0.75 * ctx.size().width);
        let width = height * (1.0 / 0.75);
        // TODO: origin?
        let x0 = 0.0;
        let y0 = 0.0;
        let _w = ctx.size().width;
        ctx.draw_image(
            &b,
            Rect::new(x0 + _w - width, y0, x0 + _w, y0 + height),
            InterpolationMode::Bilinear,
        );
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.max()
    }
}

impl Scope {
    pub fn new() -> Scope {
        let s = s::Scope::new(640, 480);
        Scope { s }
    }

    // pub fn ui(self, ui: &mut Ui) -> Id {
    //     let id = ui.add(self, &[]);
    //     ui.poke(id, &mut ScopeCommand::Start);
    //     id
    // }

    fn draw_test_pattern(&mut self) {
        let mut xylast = None;
        // sinewave!
        for i in 0..1001 {
            let h = (i as f32) * 0.001;
            let x = 640.0 * h;
            let y = 240.0 + 200.0 * (h * 50.0).sin();
            if let Some((xlast, ylast)) = xylast {
                self.s.add_line(xlast, ylast, x, y, 1.0, 2.0);
            }
            xylast = Some((x, y));
        }
    }
}
