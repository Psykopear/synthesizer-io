use druid::widget::{prelude::*, Label};
use druid::{Color, Data, MouseButton, MouseEvent, Widget};

// const BG_COLOR: Color = Color::rgb8(40, 44, 52);
const WHITE: Color = Color::rgb8(255, 255, 255);
const RED: Color = Color::rgb8(255, 0, 0);
// const BLACK: Color = Color::rgb8(0, 0, 0);

#[derive(PartialEq, Data, Clone, Default)]
pub struct SwitchState {
    pub on: bool,
    pub disabled: bool,
}

pub struct Switch {
    inner: Label<String>,
    text: String,
}

impl Switch {
    pub fn new(text: &'static str) -> Self {
        Switch {
            inner: Label::new(text),
            text: text.to_string(),
        }
    }
}

impl Widget<SwitchState> for Switch {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SwitchState, env: &Env) {
        self.inner.event(ctx, event, &mut self.text, env);
        if !data.disabled {
            match event {
                Event::MouseDown(MouseEvent {
                    button: MouseButton::Left,
                    ..
                }) => {
                    data.on = !data.on;
                    if data.on {
                        self.inner.set_text_color(RED);
                    } else {
                        self.inner.set_text_color(WHITE);
                    }
                    ctx.request_layout();
                    ctx.request_paint();
                }
                _ => (),
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: &SwitchState, _new: &SwitchState, env: &Env) {
        self.inner.update(ctx, &self.text, &self.text, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &SwitchState,
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, &self.text, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &SwitchState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, &self.text, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &SwitchState, env: &Env) {
        self.inner.paint(ctx, &self.text, &env);
    }
}
