use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Point, Rectangle, Size};

use crate::app::messages::Message;
use crate::engine::traits::TextQuad;

pub struct PageSelectionProgram {
    pub page_index: usize,
    pub zoom: f32,
    pub selected_quads: Vec<TextQuad>,
    pub search_quads: Vec<TextQuad>,
    pub active_search_quad: Option<TextQuad>,
}

impl Program<Message> for PageSelectionProgram {
    type State = bool; // is_dragging

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<Message>) {
        let cursor_position = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(Point { x, y }) = cursor_position {
                    *state = true;
                    return (
                        iced::event::Status::Captured,
                        Some(Message::StartTextSelection {
                            page_index: self.page_index,
                            x,
                            y,
                        }),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if *state {
                    if let Some(Point { x, y }) = cursor_position {
                        return (
                            iced::event::Status::Captured,
                            Some(Message::UpdateTextSelection {
                                page_index: self.page_index,
                                x,
                                y,
                            }),
                        );
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if *state {
                    *state = false;
                    return (
                        iced::event::Status::Captured,
                        Some(Message::EndTextSelection),
                    );
                }
            }
            _ => {}
        }

        (iced::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 1. Draw yellow highlights for all search matches on the page
        for quad in &self.search_quads {
            let q_x = quad.x0 * self.zoom;
            let q_y = quad.y0 * self.zoom;
            let q_w = (quad.x1 - quad.x0) * self.zoom;
            let q_h = (quad.y1 - quad.y0) * self.zoom;

            let rect_path = Path::rectangle(Point::new(q_x, q_y), Size::new(q_w, q_h));
            frame.fill(
                &rect_path,
                Color::from_rgba(1.0, 0.85, 0.0, 0.45), // Gold/Yellow
            );
        }

        // 2. Draw bright orange focus highlight for the active search match
        if let Some(ref quad) = self.active_search_quad {
            let q_x = quad.x0 * self.zoom;
            let q_y = quad.y0 * self.zoom;
            let q_w = (quad.x1 - quad.x0) * self.zoom;
            let q_h = (quad.y1 - quad.y0) * self.zoom;

            let rect_path = Path::rectangle(Point::new(q_x, q_y), Size::new(q_w, q_h));
            frame.fill(
                &rect_path,
                Color::from_rgba(1.0, 0.45, 0.0, 0.70), // Bright Orange
            );
            frame.stroke(
                &rect_path,
                Stroke::default()
                    .with_color(Color::from_rgb(1.0, 0.30, 0.0))
                    .with_width(2.0),
            );
        }

        // 3. Draw translucent blue highlight boxes for selected text quads
        for quad in &self.selected_quads {
            let q_x = quad.x0 * self.zoom;
            let q_y = quad.y0 * self.zoom;
            let q_w = (quad.x1 - quad.x0) * self.zoom;
            let q_h = (quad.y1 - quad.y0) * self.zoom;

            let rect_path = Path::rectangle(Point::new(q_x, q_y), Size::new(q_w, q_h));
            frame.fill(
                &rect_path,
                Color::from_rgba(0.25, 0.55, 0.95, 0.35),
            );
            frame.stroke(
                &rect_path,
                Stroke::default()
                    .with_color(Color::from_rgba(0.35, 0.65, 1.0, 0.60))
                    .with_width(1.0),
            );
        }

        vec![frame.into_geometry()]
    }
}