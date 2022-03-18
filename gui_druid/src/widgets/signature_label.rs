use druid::kurbo::{Line, Point, Rect};
use druid::widget::{prelude::*, Label};
use druid::Widget;

pub struct SignatureLabel {
    inner: Label<(u16, u16)>,
    start_mouse_position: f64,
    start_value: (u16, u16),
    editing: bool,
    editing_digit: usize,
}

impl SignatureLabel {
    pub fn new() -> Self {
        SignatureLabel {
            inner: Label::dynamic(|(num, den), _env| format!("{}/{}", num, den)),
            start_mouse_position: 0.,
            start_value: (4, 4),
            editing: false,
            editing_digit: 0,
        }
    }
    fn get_numerator(&self, mouse_pos: f64) -> u16 {
        self.start_value.0 + ((self.start_mouse_position - mouse_pos) / 5.).ceil() as u16
    }
    fn get_denominator(&self, mouse_pos: f64) -> u16 {
        self.start_value.1 + ((self.start_mouse_position - mouse_pos) / 5.).ceil() as u16
    }
}

impl Widget<(u16, u16)> for SignatureLabel {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (u16, u16), env: &Env) {
        match event {
            Event::MouseDown(mouse) => {
                if mouse.button.is_left() {
                    ctx.set_active(true);
                    self.start_value = *data;
                    self.start_mouse_position = mouse.pos.y;
                }
            }
            Event::MouseMove(mouse) => {
                if ctx.is_active() {
                    if mouse.pos.x > ctx.size().width / 2. {
                        data.1 = self.get_denominator(mouse.pos.y);
                    } else {
                        data.0 = self.get_numerator(mouse.pos.y);
                    }
                    ctx.request_layout();
                }
            }
            Event::MouseUp(mouse) => {
                if mouse.button.is_left() && ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &(u16, u16), new: &(u16, u16), env: &Env) {
        self.inner.update(ctx, &old, &new, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &(u16, u16),
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, &data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &(u16, u16),
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, &data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &(u16, u16), env: &Env) {
        self.inner.paint(ctx, &data, env);

        if self.editing {
            let color = env.get(druid::theme::PRIMARY_LIGHT);
            let brush = ctx.solid_brush(color.with_alpha(0.5));
            let step = ctx.size().width / 6.;
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
