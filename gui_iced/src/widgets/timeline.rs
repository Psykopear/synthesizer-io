use iced::{pure::Element, Color, Size, Point, Rectangle};
use iced_native::{layout, renderer, Layout};
use iced_pure::{Widget, widget::Tree};

pub struct Timeline {}

impl Timeline {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Message, Renderer> Widget<Message, Renderer> for Timeline
where
    Renderer: iced_native::Renderer,
{
    fn width(&self) -> iced::Length {
        iced::Length::Fill
    }

    fn height(&self) -> iced::Length {
        iced::Length::Fill
    }

    fn layout(
        &self,
        _renderer: &Renderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        let size = limits.resolve(Size::ZERO);
        layout::Node::new(size)
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) {

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color::BLACK,
        );
    }
}

impl<'a, Message> From<Timeline> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(timeline: Timeline) -> Element<'a, Message> {
        Element::new(timeline)
    }
}
