use iced::widget::{button, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::messages::Message;
use crate::models::workspace::RuntimeTab;

pub fn render_search_bar<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let text_color = Color::from_rgb(0.92, 0.94, 0.98);
    let accent_color = Color::from_rgb(0.48, 0.72, 0.98);

    let search_input = text_input("Search document...", &tab.search_query)
        .on_input(Message::SearchQueryChanged)
        .on_submit(Message::NextSearchMatch)
        .width(Length::Fixed(200.0))
        .padding([4, 8])
        .style(|_theme, status| {
            let border_color = match status {
                text_input::Status::Focused => Color::from_rgb(0.38, 0.58, 0.92),
                text_input::Status::Hovered => Color::from_rgb(0.40, 0.45, 0.55),
                _ => Color::from_rgb(0.28, 0.30, 0.36),
            };
            text_input::Style {
                background: Color::from_rgb(0.18, 0.19, 0.23).into(),
                border: iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: Color::TRANSPARENT,
                placeholder: Color::from_rgb(0.50, 0.50, 0.50),
                value: Color::from_rgb(0.95, 0.95, 0.98),
                selection: Color::from_rgb(0.25, 0.35, 0.55),
            }
        });

    let match_case_btn = button(text("Aa").size(11).color(if tab.search_match_case { accent_color } else { text_color }))
        .on_press(Message::ToggleSearchMatchCase)
        .padding([4, 8])
        .style(move |_theme, status| {
            let bg = if tab.search_match_case {
                Color::from_rgb(0.20, 0.28, 0.42)
            } else if matches!(status, button::Status::Hovered) {
                Color::from_rgb(0.28, 0.30, 0.36)
            } else {
                Color::from_rgb(0.20, 0.22, 0.26)
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if tab.search_match_case { accent_color } else { text_color },
                border: iced::Border {
                    color: if tab.search_match_case { Color::from_rgb(0.38, 0.58, 0.92) } else { Color::from_rgb(0.30, 0.32, 0.38) },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

    let total_matches = tab.search_matches.len();
    let counter_str = if total_matches > 0 {
        format!("{} / {}", tab.current_search_idx + 1, total_matches)
    } else if !tab.search_query.trim().is_empty() {
        "0 matches".to_string()
    } else {
        "".to_string()
    };

    let match_counter = text(counter_str).size(11).color(Color::from_rgb(0.70, 0.75, 0.85));

    let prev_btn = button(text("<").size(11).color(text_color))
        .on_press(Message::PrevSearchMatch)
        .padding([4, 8])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.28, 0.30, 0.36),
                _ => Color::from_rgb(0.20, 0.22, 0.26),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.92, 0.94, 0.98),
                border: iced::Border {
                    color: Color::from_rgb(0.30, 0.32, 0.38),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

    let next_btn = button(text(">").size(11).color(text_color))
        .on_press(Message::NextSearchMatch)
        .padding([4, 8])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.28, 0.30, 0.36),
                _ => Color::from_rgb(0.20, 0.22, 0.26),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.92, 0.94, 0.98),
                border: iced::Border {
                    color: Color::from_rgb(0.30, 0.32, 0.38),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

    let close_btn = button(text("✕").size(11).color(text_color))
        .on_press(Message::CloseSearch)
        .padding([4, 8])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.92, 0.94, 0.98),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let search_bar_content = row![
        search_input,
        match_case_btn,
        match_counter,
        prev_btn,
        next_btn,
        close_btn,
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    container(search_bar_content)
        .padding([6, 10])
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.12, 0.13, 0.16).into()),
            border: iced::Border {
                color: Color::from_rgb(0.30, 0.35, 0.45),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}