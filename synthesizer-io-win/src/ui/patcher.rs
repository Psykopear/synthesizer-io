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

//! Widget representing patcher surface.

use druid::piet::{Brush, LineCap, StrokeStyle, Text, TextLayout, TextLayoutBuilder};
use itertools::Itertools;

use druid::kurbo::{BezPath, Line, Rect};

// use druid::piet::{
//     FillRule, FontBuilder, LineCap, RenderContext, StrokeStyle, Text, TextLayout, TextLayoutBuilder,
// };

// use druid::widget::MouseButton;
use druid::{
    BoxConstraints, Color, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseButton,
    RenderContext, Selector, Size,
};
use druid::{LayoutCtx, PaintCtx};
use druid::{MouseEvent, Widget};

use crate::grid::{
    Delta, JumperDelta, ModuleGrid, ModuleInstance, ModuleSpec, WireDelta, WireGrid,
};

pub struct Patcher {
    size: (f32, f32),
    grid_size: (usize, usize),
    offset: (f32, f32),
    scale: f32,

    mode: PatcherMode,

    // These next are per-mode state, might want to move into mode enum.
    grid: WireGrid,
    last_xy: Option<(f32, f32)>,
    draw_mode: Option<bool>,

    modules: ModuleGrid,
    mod_hover: Option<ModuleInstance>,
    mod_name: String,

    jumper_start: Option<(u16, u16)>,
    jumper_hover: Option<(u16, u16)>,
}

#[derive(Debug)]
pub enum PatcherAction {
    WireMode,
    JumperMode,
    Module(String),
}

pub const WIRE_MODE: Selector = Selector::new("synthesizer-io.patcher.wire-mode");
pub const JUMPER_MODE: Selector = Selector::new("synthesizer-io.patcher.jumper-mode");
pub const MODULE: Selector<String> = Selector::new("synthesizer-io.patcher.module");

#[derive(PartialEq)]
enum PatcherMode {
    Wire,
    Jumper,
    Module,
}

struct PaintResources {
    grid_color: Brush,
    wire_color: Brush,
    jumper_color: Brush,
    text_color: Brush,
    hover_ok: Brush,
    hover_bad: Brush,
    module_color: Brush,
    rounded: StrokeStyle,
}

impl PaintResources {
    fn create(ctx: &mut PaintCtx) -> PaintResources {
        // PaintCtx API is awkward, can't borrow d2d_factory while render_target
        // is borrowed. This works but should be improved (by having state splitting).
        let mut rounded = StrokeStyle::new();
        rounded.set_line_cap(LineCap::Round);
        let grid_color = ctx.solid_brush(Color::Rgba32(0x405070ff));
        let wire_color = ctx.solid_brush(Color::Rgba32(0x908060ff));
        let jumper_color = ctx.solid_brush(Color::Rgba32(0x800000ff));
        let text_color = ctx.solid_brush(Color::Rgba32(0x303030ff));
        let hover_ok = ctx.solid_brush(Color::Rgba32(0x00c000ff));
        let hover_bad = ctx.solid_brush(Color::Rgba32(0xc00000ff));
        let module_color = ctx.solid_brush(Color::Rgba32(0xc0c0c0ff));
        PaintResources {
            grid_color,
            wire_color,
            jumper_color,
            text_color,
            hover_ok,
            hover_bad,
            module_color,
            rounded,
        }
    }
}

impl<T: Data> Widget<T> for Patcher {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        // TODO: retain these resources where possible
        let mut resources = PaintResources::create(ctx);
        // self.populate_text(&mut resources, ctx);
        self.paint_wiregrid(ctx, &resources);
        self.paint_modules(ctx, &resources);
        self.paint_jumpers(ctx, &resources);
        self.paint_pads(ctx, &resources);
        if self.mode == PatcherMode::Jumper {
            self.paint_jumper_hover(ctx, &resources);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.constrain((100.0, 100.0))
    }

    fn event(&mut self, ctx: &mut druid::EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(event) => {
                // Middle mouse button cycles through modes; might be obsolete
                if event.button == MouseButton::Middle {
                    if event.count > 0 {
                        let new_mode = match self.mode {
                            PatcherMode::Wire => PatcherMode::Module,
                            PatcherMode::Module => PatcherMode::Jumper,
                            PatcherMode::Jumper => PatcherMode::Wire,
                        };
                        self.mode = new_mode;
                        // self.update_hover(None, ctx);
                    }
                }
                match self.mode {
                    PatcherMode::Wire => {
                        if event.count > 0 {
                            self.last_xy = Some((event.pos.x as f32, event.pos.y as f32));
                            self.draw_mode = None;
                            ctx.set_active(true);
                        } else {
                            self.last_xy = None;
                            ctx.set_active(false);
                        }
                    }
                    PatcherMode::Module => {
                        if let Some(mut inst) = self.mod_hover.take() {
                            // TODO: reduce dupl
                            let xc = event.pos.x as f32 - 0.5 * self.scale * (inst.spec.size.0 as f32 - 1.0);
                            let yc = event.pos.y as f32 - 0.5 * self.scale * (inst.spec.size.1 as f32 - 1.0);
                            if let Some(loc) = self.xy_to_cell(xc, yc) {
                                inst.loc = loc;
                                if self.is_module_ok(&inst) {
                                    let delta = vec![Delta::Module(inst)];
                                    self.apply_and_send_delta(delta, ctx);
                                    /*
                                    println!("placing {} at {:?}", inst.spec.name, inst.loc);
                                    self.modules.add(inst);
                                    ctx.send_event(vec![Delta::Module]);
                                    ctx.invalidate();
                                    */
                                }
                            }
                        }
                    }
                    PatcherMode::Jumper => {
                        if event.count > 0 {
                            if let Some(start) = self.jumper_start.take() {
                                if let Some(end) = self.jumper_hover {
                                    if start != end {
                                        let jumper_delta = JumperDelta {
                                            start,
                                            end,
                                            val: true,
                                        };
                                        let delta = vec![Delta::Jumper(jumper_delta)];
                                        self.apply_and_send_delta(delta, ctx);
                                    }
                                }
                            } else {
                                self.jumper_start = self.jumper_hover;
                            }
                            ctx.request_paint();
                        }
                    }
                }
            }
            Event::MouseMove(event) => {
                match self.mode {
                    PatcherMode::Wire => {
                        if let Some((x0, y0)) = self.last_xy {
                            let mut delta = Vec::new();
                            let pts =
                                self.line_quantize(x0, y0, event.pos.x as f32, event.pos.y as f32);
                            for ((x0, y0), (x1, y1)) in pts.iter().tuple_windows() {
                                let grid_ix = WireGrid::unit_line_to_grid_ix(*x0, *y0, *x1, *y1);
                                if self.draw_mode.is_none() {
                                    self.draw_mode = Some(!self.grid.is_set(grid_ix));
                                }
                                let val = self.draw_mode.unwrap();
                                delta.push(Delta::Wire(WireDelta { grid_ix, val }));
                            }
                            self.apply_and_send_delta(delta, ctx);
                            self.last_xy = Some((event.pos.x as f32, event.pos.y as f32))
                        }
                    }
                    PatcherMode::Module => {
                        // could reduce the allocation here, but no biggie
                        let spec = if let Some(ref h) = self.mod_hover {
                            h.spec.clone()
                        } else {
                            make_mod_spec(&self.mod_name)
                        };
                        let xc = event.pos.x as f32 - 0.5 * self.scale * (spec.size.0 as f32 - 1.0);
                        let yc = event.pos.y as f32 - 0.5 * self.scale * (spec.size.1 as f32 - 1.0);
                        let instance = self
                            .xy_to_cell(xc, yc)
                            .map(|loc| ModuleInstance { loc, spec });
                        // self.update_hover(instance, ctx);
                    }
                    PatcherMode::Jumper => {
                        // NYI
                        self.jumper_hover = self.xy_to_cell(event.pos.x as f32, event.pos.y as f32);
                        ctx.request_paint();
                    }
                }
            }
            Event::Command(cmd) => {
                if let Some(_) = cmd.get(WIRE_MODE) {
                    self.mode = PatcherMode::Wire;
                }
                if let Some(_) = cmd.get(JUMPER_MODE) {
                    self.mode = PatcherMode::Jumper;
                }
                if let Some(name) = cmd.get(MODULE) {
                    self.mode = PatcherMode::Module;
                    self.mod_name = name.clone();
                }
                // self.update_hover(None, ctx);
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, old_data: &T, data: &T, env: &Env) {}

    // fn on_hot_changed(&mut self, hot: bool, ctx: &mut HandlerCtx) {
    //     if !hot {
    //         self.update_hover(None, ctx);
    //     }
    // }
}

impl Patcher {
    pub fn new() -> Patcher {
        Patcher {
            size: (0.0, 0.0),
            grid_size: (20, 16),
            offset: (5.0, 5.0),
            scale: 20.0,

            mode: PatcherMode::Wire,

            grid: Default::default(),
            last_xy: None,
            draw_mode: None,

            modules: Default::default(),
            mod_hover: None,
            mod_name: Default::default(),

            jumper_start: None,
            jumper_hover: None,
        }
    }

    fn paint_wiregrid(&mut self, ctx: &mut PaintCtx, resources: &PaintResources) {
        // TODO: clip to geom
        //rt.push_axis_aligned_clip(geom, AntialiasMode::Aliased);
        let x0 = self.offset.0;
        let y0 = self.offset.1;
        for i in 0..(self.grid_size.0 + 1) {
            ctx.stroke(
                line(
                    (x0 + self.scale * (i as f32), y0),
                    (
                        x0 + self.scale * (i as f32),
                        y0 + self.scale * (self.grid_size.1 as f32),
                    ),
                ),
                &resources.grid_color,
                1.0,
            );
        }
        for i in 0..(self.grid_size.1 + 1) {
            ctx.stroke(
                line(
                    (x0, y0 + self.scale * (i as f32)),
                    (
                        x0 + self.scale * (self.grid_size.0 as f32),
                        y0 + self.scale * (i as f32),
                    ),
                ),
                &resources.grid_color,
                1.0,
            );
        }
        for (i, j, vert) in self.grid.iter() {
            let x = x0 + (*i as f32 + 0.5) * self.scale;
            let y = y0 + (*j as f32 + 0.5) * self.scale;
            let (x1, y1) = if *vert {
                (x, y + self.scale)
            } else {
                (x + self.scale, y)
            };
            ctx.stroke(
                line((x, y), (x1, y1)),
                &resources.wire_color,
                3.0,
                // Some(&resources.rounded),
            );
        }
    }

    fn paint_jumpers(&mut self, ctx: &mut PaintCtx, resources: &PaintResources) {
        // let x = geom.pos.0 + self.offset.0;
        // let y = geom.pos.1 + self.offset.1;
        let x = self.offset.0;
        let y = self.offset.1;
        for (i0, j0, i1, j1) in self.grid.iter_jumpers() {
            let x0 = x + (*i0 as f32 + 0.5) * self.scale;
            let y0 = y + (*j0 as f32 + 0.5) * self.scale;
            let x1 = x + (*i1 as f32 + 0.5) * self.scale;
            let y1 = y + (*j1 as f32 + 0.5) * self.scale;
            let s = 0.3 * self.scale / (x1 - x0).hypot(y1 - y0);
            let xu = (x1 - x0) * s;
            let yu = (y1 - y0) * s;
            ctx.stroke(line((x0, y0), (x1, y1)), &resources.wire_color, 2.0);
            let r = self.scale * 0.15;
            ctx.fill(
                circle((x0, y0), r, 16),
                &resources.wire_color,
                // FillRule::NonZero,
            );
            ctx.fill(
                circle((x1, y1), r, 16),
                &resources.wire_color,
                // FillRule::NonZero,
            );
            ctx.stroke(
                line((x0 + xu, y0 + yu), (x1 - xu, y1 - yu)),
                &resources.jumper_color,
                4.0,
                // None,
            );
        }
    }

    fn paint_modules(&mut self, ctx: &mut PaintCtx, resources: &PaintResources) {
        for inst in self.modules.iter() {
            self.paint_module(ctx, resources, inst);
        }
        if let Some(ref inst) = self.mod_hover {
            let (i, j) = inst.loc;
            let (w, h) = inst.spec.size;
            // let x0 = geom.pos.0 + self.offset.0;
            // let y0 = geom.pos.1 + self.offset.1;
            let x0 = self.offset.0;
            let y0 = self.offset.1;
            let color = if self.is_module_ok(inst) {
                &resources.hover_ok
            } else {
                &resources.hover_bad
            };
            ctx.fill(
                Rect::new(
                    (x0 + (i as f32) * self.scale) as f64,
                    (y0 + (j as f32) * self.scale) as f64,
                    (x0 + ((i + w) as f32) * self.scale) as f64,
                    (y0 + ((j + h) as f32) * self.scale) as f64,
                ),
                color,
                // FillRule::NonZero,
            );
        }
    }

    fn paint_module(&self, ctx: &mut PaintCtx, resources: &PaintResources, inst: &ModuleInstance) {
        // let x0 = geom.pos.0 + self.offset.0 + (inst.loc.0 as f32) * self.scale;
        // let y0 = geom.pos.1 + self.offset.1 + (inst.loc.1 as f32) * self.scale;
        let x0 = self.offset.0 + (inst.loc.0 as f32) * self.scale;
        let y0 = self.offset.1 + (inst.loc.1 as f32) * self.scale;
        let inset = 0.1;
        ctx.fill(
            Rect::new(
                (x0 + inset * self.scale) as f64,
                (y0 + inset * self.scale) as f64,
                (x0 + (inst.spec.size.0 as f32 - inset) * self.scale) as f64,
                (y0 + (inst.spec.size.1 as f32 - inset) * self.scale) as f64,
            ),
            &resources.module_color,
            // FillRule::NonZero,
        );
        if inst.spec.name == "control" {
            return;
        }
        for j in 0..inst.spec.size.1 {
            let xl = x0 + inset * self.scale;
            let xr = x0 + (inst.spec.size.0 as f32 - inset) * self.scale;
            let y = y0 + (j as f32 + 0.5) * self.scale;
            let width = 2.0;
            ctx.stroke(
                line((xl, y), (xl - (0.5 + inset) * self.scale, y)),
                &resources.module_color,
                width,
                // None,
            );
            ctx.stroke(
                line((xr, y), (xr + (0.5 + inset) * self.scale, y)),
                &resources.module_color,
                width,
                // None,
            );
        }
        // let layout = &resources.text[&inst.spec.name];
        let layout = ctx.text().new_text_layout(inst.spec.name.clone()).build().unwrap();
        // TODO
        // let text_width = layout.layout_metrics().size.width;
        let text_width = 10.0;
        let text_x = x0 + 0.5 * ((inst.spec.size.0 as f32) * self.scale - text_width);
        ctx.draw_text(&layout, (text_x as f64, y0 as f64 + 12.0));
    }

    fn paint_jumper_hover(&self, ctx: &mut PaintCtx, resources: &PaintResources) {
        if let Some((i, j)) = self.jumper_hover {
            // let xc = geom.pos.0 + self.offset.0 + (i as f32 + 0.5) * self.scale;
            // let yc = geom.pos.1 + self.offset.1 + (j as f32 + 0.5) * self.scale;
            let xc = self.offset.0 + (i as f32 + 0.5) * self.scale;
            let yc = self.offset.1 + (j as f32 + 0.5) * self.scale;
            let r = self.scale * 0.15;
            if let Some((i, j)) = self.jumper_start {
                // let xsc = geom.pos.0 + self.offset.0 + (i as f32 + 0.5) * self.scale;
                // let ysc = geom.pos.1 + self.offset.1 + (j as f32 + 0.5) * self.scale;
                let xsc = self.offset.0 + (i as f32 + 0.5) * self.scale;
                let ysc = self.offset.1 + (j as f32 + 0.5) * self.scale;
                let r = self.scale * 0.15;
                ctx.stroke(line((xsc, ysc), (xc, yc)), &resources.wire_color, 1.5);
                ctx.fill(
                    circle((xsc, ysc), r, 16),
                    &resources.hover_ok,
                    // FillRule::NonZero,
                );
            }
            ctx.fill(
                circle((xc, yc), r, 16),
                &resources.hover_ok,
                // FillRule::NonZero,
            );
        }
    }

    fn paint_pads(&self, ctx: &mut PaintCtx, resources: &PaintResources) {
        // let x0 = geom.pos.0 + self.offset.0 + (self.grid_size.0 as f32 - 0.5) * self.scale;
        // let y0 = geom.pos.1 + self.offset.1 + (self.grid_size.1 as f32 - 0.5) * self.scale;
        let x0 = self.offset.0 + (self.grid_size.0 as f32 - 0.5) * self.scale;
        let y0 = self.offset.1 + (self.grid_size.1 as f32 - 0.5) * self.scale;
        let layout = &ctx.text().new_text_layout("\u{1F50A}").build().unwrap();
        ctx.draw_text(
            layout,
            (
                x0 as f64 + 0.6 * self.scale as f64,
                y0 as f64 - 0.4 * self.scale as f64 + 12.0,
            ),
            // &resources.text_color,
        );

        ctx.stroke(
            line((x0, y0), (x0 + 0.6 * self.scale, y0)),
            &resources.wire_color,
            3.0,
            // Some(&resources.rounded),
        );
    }

    // TODO: could take text factory. Rethink lifetimes
    // fn populate_text(&self, resources: &mut PaintResources, rc: &mut Piet) {
    //     for inst in self.modules.iter() {
    //         resources.add_text(&inst.spec.name, rc);
    //     }
    //     resources.add_text("\u{1F50A}", rc);
    // }

    fn xy_to_cell(&self, x: f32, y: f32) -> Option<(u16, u16)> {
        let u = (x - self.offset.0) / self.scale;
        let v = (y - self.offset.1) / self.scale;
        self.guard_point(u, v)
    }

    // TODO: This is not necessarily the absolute perfect logic, but it should work.
    fn line_quantize(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(u16, u16)> {
        let mut u = (x0 - self.offset.0) / self.scale;
        let mut v = (y0 - self.offset.1) / self.scale;
        let u1 = (x1 - self.offset.0) / self.scale;
        let v1 = (y1 - self.offset.1) / self.scale;
        let du = u1 - u;
        let dv = v1 - v;
        let mut vec = Vec::new();
        vec.extend(self.guard_point(u, v));
        let mut last_u = u.floor();
        let mut last_v = v.floor();
        if du.abs() > dv.abs() {
            while u.floor() != u1.floor() {
                let new_u = if du > 0.0 {
                    u.floor() + 1.0
                } else {
                    u.ceil() - 1.0
                };
                if new_u.floor() != last_u {
                    vec.extend(self.guard_point(new_u, last_v));
                }
                v += (new_u - u) * dv / du;
                u = new_u;
                if v.floor() != last_v {
                    vec.extend(self.guard_point(u, v));
                }
                last_u = u.floor();
                last_v = v.floor();
            }
        } else {
            while v.floor() != v1.floor() {
                let new_v = if dv > 0.0 {
                    v.floor() + 1.0
                } else {
                    v.ceil() - 1.0
                };
                if new_v.floor() != last_v {
                    vec.extend(self.guard_point(last_u, new_v));
                }
                u += (new_v - v) * du / dv;
                v = new_v;
                if u.floor() != last_u {
                    vec.extend(self.guard_point(u, v));
                }
                last_u = u.floor();
                last_v = v.floor();
            }
        }
        if u1.floor() != last_u || v1.floor() != last_v {
            vec.extend(self.guard_point(u1, v1));
        }
        vec
    }

    fn guard_point(&self, u: f32, v: f32) -> Option<(u16, u16)> {
        let i = u.floor() as isize;
        let j = v.floor() as isize;
        if i >= 0 && j >= 0 && (i as usize) < self.grid_size.0 && (j as usize) < self.grid_size.1 {
            Some((i as u16, j as u16))
        } else {
            None
        }
    }

    // fn update_hover(&mut self, hover: Option<ModuleInstance>, ctx: &mut HandlerCtx) {
    //     if self.mod_hover != hover {
    //         self.mod_hover = hover;
    //         ctx.invalidate();
    //     }
    // }

    fn is_module_ok(&self, inst: &ModuleInstance) -> bool {
        !self.modules.is_conflict(inst)
    }

    fn apply_and_send_delta(&mut self, delta: Vec<Delta>, ctx: &mut EventCtx) {
        if !delta.is_empty() {
            self.apply_delta(&delta);
            // ctx.send_event(delta);
            // ctx.invalidate();
        }
    }

    fn apply_delta(&mut self, delta: &[Delta]) {
        for d in delta {
            match d {
                Delta::Wire(WireDelta { grid_ix, val }) => {
                    self.grid.set(*grid_ix, *val);
                }
                Delta::Jumper(delta) => {
                    self.grid.apply_jumper_delta(delta.clone());
                }
                Delta::Module(inst) => {
                    self.modules.add(inst.clone());
                }
            }
        }
    }
}

/// Make a module spec given a name.
///
/// This will probably grow into a registry, but for now can be basically
/// hard-coded.
fn make_mod_spec(name: &str) -> ModuleSpec {
    let size = match name {
        "sine" | "saw" => (2, 1),
        "adsr" => (2, 3),
        "control" => (1, 1),
        _ => (2, 2),
    };
    ModuleSpec {
        size,
        name: name.into(),
    }
}

// TODO: should Line::new in kurbo auto-cast this?
fn line(p0: (f32, f32), p1: (f32, f32)) -> Line {
    Line::new((p0.0 as f64, p0.1 as f64), (p1.0 as f64, p1.1 as f64))
}

// TODO: this will eventually become a `kurbo::Shape`.
fn circle(center: (f32, f32), radius: f32, num_segments: usize) -> BezPath {
    let mut path = BezPath::new();
    if num_segments == 0 {
        return path;
    }

    let radius = radius as f64;
    let centerx = center.0 as f64;
    let centery = center.1 as f64;
    for segment in 0..num_segments {
        let theta = 2.0 * std::f64::consts::PI * (segment as f64) / (num_segments as f64);
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        if segment == 0 {
            path.move_to((x + centerx, y + centery));
        } else {
            let end = (x + centerx, y + centery);
            path.line_to(end);
        }
    }

    path.close_path();
    return path;
}
