use druid::kurbo::{Line, Point, Rect};
use druid::widget::{prelude::*, Label};
use druid::{Widget, WidgetExt};

pub struct TempoLabel {
    inner: Box<dyn Widget<f64>>,
    start_mouse_position: f64,
    start_value: f64,
    precise: bool,
    editing: bool,
    editing_digit: usize,
}

impl TempoLabel {
    pub fn new() -> Self {
        TempoLabel {
            inner: Self::make_ui().boxed(),
            start_mouse_position: 0.,
            start_value: 0.,
            precise: false,
            editing: false,
            editing_digit: 0,
        }
    }

    fn make_ui() -> impl Widget<f64> {
        Label::dynamic(|data, _env| format!("{:.2}", data))
    }
}

impl Widget<f64> for TempoLabel {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        match event {
            Event::KeyDown(key) => {
//                 if key.key_code == druid::KeyCode::Return {
//                     self.editing = false;
//                     ctx.request_paint();
//                 };
//                 if key.key_code.is_printable() {
//                     if let Some(chars) = key.text() {
//                         if let Ok(number) = chars.parse::<u32>() {
//                             let result: String = format!("{:.2}", data)
//                                 .chars()
//                                 .enumerate()
//                                 .map(|(i, x)| {
//                                     if i == self.editing_digit {
//                                         std::char::from_digit(number, 10).unwrap()
//                                     } else {
//                                         x
//                                     }
//                                 })
//                                 .collect();
//
//                             *data = result.parse::<f64>().unwrap();
//                             if self.editing_digit != 2 {
//                                 self.editing_digit += 1;
//                             } else {
//                                 self.editing_digit += 2;
//                             }
//                             if self.editing_digit == 6 {
//                                 self.editing = false;
//                             }
//                             ctx.request_paint();
//                         }
//                     }
//                 }
            }

            Event::MouseDown(mouse) => {
                if mouse.button.is_left() {
                    ctx.set_active(true);
                    self.start_value = *data;
                    self.start_mouse_position = mouse.pos.y;
                } else if mouse.button.is_right() {
                    ctx.set_active(true);
                    self.start_value = *data;
                    self.start_mouse_position = mouse.pos.y;
                    self.precise = true;
                } else if mouse.button.is_middle() {
                    ctx.request_focus();
                    self.editing = true;
                    self.editing_digit = 0;
                }
            }
            Event::MouseMove(mouse) => {
                if ctx.is_active() {
                    if self.precise {
                        *data = self.start_value
                            + ((self.start_mouse_position - mouse.pos.y) / 5.).ceil() / 100.;
                    } else {
                        *data = self.start_value
                            + ((self.start_mouse_position - mouse.pos.y) / 5.).ceil();
                    }
                    ctx.request_layout();
                }
            }
            Event::MouseUp(mouse) => {
                if mouse.button.is_left() && ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
                if mouse.button.is_right() && ctx.is_active() {
                    self.precise = false;
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &f64, new: &f64, env: &Env) {
        self.inner.update(ctx, &old, &new, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &f64, env: &Env) {
        self.inner.lifecycle(ctx, event, &data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &f64, env: &Env) -> Size {
        self.inner.layout(ctx, bc, &data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        self.inner.paint(ctx, &data, env);
        if self.editing {
            let color = env.get(druid::theme::PRIMARY_LIGHT);
            let brush = ctx.solid_brush(color.with_alpha(0.5));
            let step = ctx.size().width / 6.2;
            let size = druid::kurbo::Size::new(step, ctx.size().height);
            let mut origin = Point::ORIGIN;
            origin.x = step * self.editing_digit as f64;
            ctx.fill(Rect::from_origin_size(origin, size), &brush);
        }
        if self.editing || ctx.is_active() {
            let color = env.get(druid::theme::PRIMARY_LIGHT);
            let brush = ctx.solid_brush(color);
            let start = Point::new(0. - 8., ctx.size().height + 2.);
            let end = Point::new(ctx.size().width + 10., ctx.size().height + 2.);
            ctx.stroke(Line::new(start, end), &brush, 2.);
        }
    }
}
