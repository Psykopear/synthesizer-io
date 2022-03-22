use druid::widget::{prelude::*, Label};
use druid::{Color, Data, MouseButton, MouseEvent, Widget};

// const BG_COLOR: Color = Color::rgb8(40, 44, 52);
const WHITE: Color = Color::rgb8(255, 255, 255);
const RED: Color = Color::rgb8(255, 0, 0);
const GRAY: Color = Color::rgb8(50, 100, 50);
const BLACK: Color = Color::rgb8(0, 0, 0);

#[derive(PartialEq, Data, Clone, Default)]
pub struct SwitchState {
    pub on: bool,
    pub disabled: bool,
}

pub struct Switch {
    inner: Label<SwitchState>,
}

impl Switch {
    pub fn new(text: &'static str) -> Self {
        Switch {
            inner: Label::new(text),
        }
    }

    pub fn switch(&mut self, state: &SwitchState) {
        if state.disabled {
            self.inner.set_text_color(GRAY);
        } else {
            if state.on {
                self.inner.set_text_color(RED);
            } else {
                self.inner.set_text_color(WHITE);
            }
        }
    }
}

impl Widget<SwitchState> for Switch {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SwitchState, env: &Env) {
        if !data.disabled {
            match event {
                Event::MouseDown(MouseEvent {
                    button: MouseButton::Left,
                    ..
                }) => {
                    data.on = !data.on;
                    self.switch(data);
                    ctx.request_layout();
                }
                _ => (),
            }
        }
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &SwitchState, new: &SwitchState, env: &Env) {
        self.inner.update(ctx, old, new, env);
        if old != new {
            self.switch(new);
            ctx.request_layout();
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &SwitchState,
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, _data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &SwitchState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, _data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &SwitchState, env: &Env) {
        self.inner.paint(ctx, data, &env);
    }
}
