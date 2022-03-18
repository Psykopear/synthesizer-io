use druid::widget::prelude::*;
use druid::{Color, Data, MouseButton, MouseEvent, Widget, WidgetExt, WidgetPod};

const BG_COLOR: Color = Color::rgb8(40, 44, 52);

#[derive(PartialEq, Data, Clone, Default)]
pub struct SwitchState {
    pub on: bool,
    pub disabled: bool,
}

pub struct Switch {
    inner: WidgetPod<String, Box<dyn Widget<String>>>,
    text: String,
}

impl Switch {
    pub fn new(text: &'static str) -> Self {
        Switch {
            inner: WidgetPod::new(
                druid::widget::Label::new(text)
                    .with_text_size(24.)
                    .padding(4.)
                    .boxed(),
            ),
            text: text.to_string(),
        }
    }
}

impl Widget<SwitchState> for Switch {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut SwitchState, _env: &Env) {
        if !data.disabled {
            match event {
                Event::MouseDown(MouseEvent {
                    button: MouseButton::Left,
                    ..
                }) => {
                    data.on = !data.on;
                }
                _ => (),
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &SwitchState, new: &SwitchState, _env: &Env) {
        if old != new {
            ctx.request_paint();
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &SwitchState,
        _env: &Env,
    ) {
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
        let mut env = env.to_owned();
        if data.disabled {
            env.set(druid::theme::TEXT_COLOR, BG_COLOR);
        } else if data.on {
            env.set(
                druid::theme::TEXT_COLOR,
                env.get(druid::theme::PRIMARY_DARK),
            );
        }
        self.inner.paint_raw(ctx, &self.text, &env);
    }
}
