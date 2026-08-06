use iced::widget::{button, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::messages::Message;
use crate::models::workspace::RuntimeTab;
use crate::ui::theme::invisible_scrollable_style;

pub fn render_tab_bar<'a>(
    tabs: &'a [RuntimeTab],
    active_id: usize,
    dragged_tab_id: Option<usize>,
) -> Element<'a, Message> {
    let mut tab_row = row![].spacing(6).align_y(Alignment::Center);

    for tab in tabs {
        let is_active = tab.id == active_id;
        let is_dragging = dragged_tab_id == Some(tab.id);
        let tab_id = tab.id;

        let display_title = if tab.title.len() > 18 {
            format!("{}…", &tab.title[..16])
        } else {
            tab.title.clone()
        };

        // Plain styled text element (no inner button so mouse_area receives clicks)
        let title_label = text(display_title)
            .size(13)
            .color(if is_active {
                Color::from_rgb(1.0, 1.0, 1.0)
            } else {
                Color::from_rgb(0.8, 0.85, 0.9)
            });

        let title_container = container(title_label).padding([4.0, 6.0]);

        let close_btn = button(text("✕").size(10))
            .on_press(Message::CloseTab(tab_id))
            .padding([3.0, 6.0])
            .style(move |_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Color::from_rgb(0.9, 0.92, 0.95),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let tab_inner_row = row![title_container, close_btn]
            .spacing(2)
            .align_y(Alignment::Center);

        let tab_card = container(tab_inner_row)
            .padding([1.0, 4.0])
            .style(move |_theme| {
                let (bg_color, border_color, border_width) = if is_dragging {
                    (
                        Color::from_rgb(0.32, 0.42, 0.58),
                        Color::from_rgb(0.48, 0.68, 0.98),
                        2.0,
                    )
                } else if is_active {
                    (
                        Color::from_rgb(0.22, 0.28, 0.38),
                        Color::from_rgb(0.38, 0.58, 0.92),
                        1.0,
                    )
                } else {
                    (
                        Color::from_rgb(0.18, 0.19, 0.22),
                        Color::from_rgb(0.3, 0.32, 0.38),
                        1.0,
                    )
                };

                container::Style {
                    background: Some(bg_color.into()),
                    border: iced::Border {
                        color: border_color,
                        width: border_width,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            });

        // mouse_area receives press events anywhere on the tab card
        let draggable_tab = mouse_area(tab_card)
            .on_press(Message::StartTabDrag(tab_id))
            .on_enter(Message::TabDraggedOver(tab_id));

        tab_row = tab_row.push(draggable_tab);
    }

    let new_tab_btn = button(text("+").size(15))
        .on_press(Message::OpenFileRequested)
        .padding([5.0, 10.0])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.28, 0.3, 0.35),
                _ => Color::from_rgb(0.18, 0.19, 0.22),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.9, 0.92, 0.95),
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.32, 0.38),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        });

    tab_row = tab_row.push(new_tab_btn);

    let scrollable_tab_strip = scrollable(tab_row)
        .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
        .style(invisible_scrollable_style)
        .height(Length::Shrink);

    container(scrollable_tab_strip)
        .width(Length::Fill)
        .padding([6.0, 10.0])
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.12, 0.13, 0.15).into()),
            border: iced::Border {
                color: Color::from_rgb(0.2, 0.22, 0.25),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}