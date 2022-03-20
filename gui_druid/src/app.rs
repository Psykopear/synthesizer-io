use druid::{widget::Flex, AppDelegate, Widget, WidgetExt};

use crate::{blocks::tempo, AppState};

pub struct Delegate {}

impl AppDelegate<AppState> for Delegate {
    fn event(
        &mut self,
        ctx: &mut druid::DelegateCtx,
        window_id: druid::WindowId,
        event: druid::Event,
        data: &mut AppState,
        env: &druid::Env,
    ) -> Option<druid::Event> {
        Some(event)
    }

    fn command(
        &mut self,
        ctx: &mut druid::DelegateCtx,
        target: druid::Target,
        cmd: &druid::Command,
        data: &mut AppState,
        env: &druid::Env,
    ) -> druid::Handled {
        druid::Handled::No
    }

    fn window_added(
        &mut self,
        id: druid::WindowId,
        handle: druid::WindowHandle,
        data: &mut AppState,
        env: &druid::Env,
        ctx: &mut druid::DelegateCtx,
    ) {
    }

    fn window_removed(
        &mut self,
        id: druid::WindowId,
        data: &mut AppState,
        env: &druid::Env,
        ctx: &mut druid::DelegateCtx,
    ) {
    }
}

pub struct App {}

impl App {
    pub fn build_ui() -> impl Widget<AppState> {
        Flex::column()
            .with_child(tempo().lens(AppState::tempo))
            .with_flex_spacer(1.)
    }
}
