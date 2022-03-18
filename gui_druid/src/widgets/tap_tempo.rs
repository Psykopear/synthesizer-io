use druid::kurbo::Line;
use druid::widget::{prelude::*, Label};
use druid::{Data, Point, Widget};

pub struct TapTempo<T> {
    inner: Label<T>,
    callback: Box<dyn Fn(&mut T, f64)>,
    beats: Vec<std::time::Instant>,
    beating: bool,
}

impl<T: Data> TapTempo<T> {
    pub fn new(callback: impl Fn(&mut T, f64) + 'static) -> Self {
        TapTempo {
            inner: Label::new("TAP"),
            callback: Box::new(callback),
            beats: vec![],
            beating: false,
        }
    }
}

impl<T: Data> Widget<T> for TapTempo<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(_) => {
                self.beating = true;
                ctx.request_paint();
                ctx.request_layout();
                if self.beats.len() == 0 {
                    // println!("First beat");
                    let now = std::time::Instant::now();
                    self.beats.push(now);
                } else if self.beats.len() < 2 {
                    let last_elapsed = self.beats.last().unwrap().elapsed().as_secs_f64();
                    if last_elapsed > 1. || last_elapsed < 0.01 {
                        // println!("Reset beat");
                        self.beats = vec![std::time::Instant::now()];
                    } else {
                        // println!("Second beat");
                        self.beats.push(std::time::Instant::now());
                    }
                }
            }
            Event::MouseUp(_) => {
                self.beating = false;
                ctx.request_paint();
                ctx.request_layout();
                if self.beats.len() == 2 {
                    let beat1 = self.beats[0].elapsed().as_secs_f64();
                    let beat2 = self.beats[1].elapsed().as_secs_f64();
                    let value = 60. / (beat1 - beat2);
                    self.callback.as_ref()(data, value);
                    self.beats = vec![];
                }
            }
            _ => (),
        };
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, new: &T, env: &Env) {
        self.inner.update(ctx, &old, &new, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, &data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, &data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, &data, env);
        if self.beating {
            let brush = ctx.solid_brush(env.get(druid::theme::PRIMARY_LIGHT));
            let start = Point::new(0. - 8., ctx.size().height + 2.);
            let end = Point::new(ctx.size().width + 8., ctx.size().height + 2.);
            ctx.stroke(Line::new(start, end), &brush, 2.);
        };
    }
}
