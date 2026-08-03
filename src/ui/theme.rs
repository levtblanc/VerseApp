use iced::widget::scrollable;
use iced::{Background, Border, Color, Theme};
use crate::models::session::ThemeMode;

pub fn get_iced_theme(mode: &ThemeMode) -> Theme {
    match mode {
        ThemeMode::Light => Theme::Light,
        ThemeMode::Dark => Theme::Dark,
    }
}

pub fn transparent_scrollable_style(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let scroller_alpha = match status {
        scrollable::Status::Hovered { .. } => 0.50,
        scrollable::Status::Dragged { .. } => 0.75,
        _ => 0.25,
    };

    let scroller = scrollable::Scroller {
        color: Color::from_rgba(0.9, 0.92, 0.98, scroller_alpha),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
    };

    let rail = scrollable::Rail {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border::default(),
        scroller,
    };

    scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
    }
}