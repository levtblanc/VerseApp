use iced::widget::scrollable;
use iced::{Background, Border, Color, Theme};
use crate::models::session::ThemeMode;

pub fn get_iced_theme(mode: &ThemeMode) -> Theme {
    match mode {
        ThemeMode::Light => Theme::Light,
        ThemeMode::Dark => Theme::Dark,
    }
}

/// Fixed Dark Translucent Scrollbar Style (Used for side panel across all themes)
pub fn dark_transparent_scrollable_style(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let (scroller_alpha, border_alpha) = match status {
        scrollable::Status::Hovered { .. } => (0.85, 0.40),
        scrollable::Status::Dragged { .. } => (1.00, 0.60),
        _ => (0.60, 0.25),
    };

    let scroller_color = Color::from_rgba(0.90, 0.92, 0.98, scroller_alpha);
    let border_color = Color::from_rgba(1.0, 1.0, 1.0, border_alpha);

    let scroller = scrollable::Scroller {
        color: scroller_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
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

/// Translucent Floating Scrollbar Style with Theme-Adaptive Colors
pub fn transparent_scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let palette = theme.palette();
    let is_dark_theme = palette.background.r < 0.5;

    let (scroller_alpha, border_alpha) = match status {
        scrollable::Status::Hovered { .. } => (0.85, 0.40),
        scrollable::Status::Dragged { .. } => (1.00, 0.60),
        _ => (0.60, 0.25),
    };

    let scroller_color = if is_dark_theme {
        Color::from_rgba(0.90, 0.92, 0.98, scroller_alpha)
    } else {
        Color::from_rgba(0.15, 0.16, 0.20, scroller_alpha)
    };

    let border_color = if is_dark_theme {
        Color::from_rgba(1.0, 1.0, 1.0, border_alpha)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, border_alpha)
    };

    let scroller = scrollable::Scroller {
        color: scroller_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
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

/// 100% Invisible Scrollbar Style (Used for horizontal tab bar)
pub fn invisible_scrollable_style(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let scroller = scrollable::Scroller {
        color: Color::TRANSPARENT,
        border: Border::default(),
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