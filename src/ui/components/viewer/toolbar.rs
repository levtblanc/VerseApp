use iced::widget::{button, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::messages::Message;
use crate::models::session::{PageLayout, ThemeMode};
use crate::models::workspace::RuntimeTab;

pub fn control_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.26, 0.28, 0.34),
        button::Status::Pressed => Color::from_rgb(0.18, 0.20, 0.25),
        _ => Color::from_rgb(0.20, 0.22, 0.26),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: Color::from_rgb(0.90, 0.92, 0.96),
        border: iced::Border {
            color: Color::from_rgb(0.30, 0.32, 0.38),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn page_input_style(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
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
}

pub fn render_control_tray<'a>(
    tab: &'a RuntimeTab,
    theme_mode: ThemeMode,
    is_night_mode: bool,
) -> Element<'a, Message> {
    let tab_id = tab.id;
    let label_color = Color::from_rgb(0.90, 0.92, 0.96);

    let prev_btn = button(text("<").size(13).color(label_color))
        .on_press(Message::ChangePage(tab_id, tab.current_page.saturating_sub(1)))
        .padding([4, 10])
        .style(control_button_style);

    // Assign unique text_input ID per tab to prevent widget state bleed
    let page_input_field = text_input("", &tab.page_input_text)
        .id(text_input::Id::new(format!("page_input_{}", tab_id)))
        .on_input(move |val| Message::PageInputChanged(tab_id, val))
        .on_submit(Message::PageInputSubmitted(tab_id))
        .width(Length::Fixed(48.0))
        .padding([2, 4])
        .align_x(Alignment::Center)
        .style(page_input_style);

    let page_indicator = row![
        page_input_field,
        text(format!(" / {}", tab.page_count.max(1)))
            .size(13)
            .color(label_color)
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let next_btn = button(text(">").size(13).color(label_color))
        .on_press(Message::ChangePage(tab_id, (tab.current_page + 1).min(tab.page_count.saturating_sub(1))))
        .padding([4, 10])
        .style(control_button_style);

    let zoom_out_btn = button(text("-").size(13).color(label_color))
        .on_press(Message::ChangeZoom(tab_id, (tab.zoom - 0.15).max(0.2)))
        .padding([4, 10])
        .style(control_button_style);

    let zoom_pct = text(format!("{}%", (tab.zoom * 100.0) as u32))
        .size(13)
        .color(label_color);

    let zoom_in_btn = button(text("+").size(13).color(label_color))
        .on_press(Message::ChangeZoom(tab_id, tab.zoom + 0.15))
        .padding([4, 10])
        .style(control_button_style);

    let layout_label = match tab.layout {
        PageLayout::Single => "Single",
        PageLayout::Double => "Double",
    };
    let layout_btn = button(text(layout_label).size(12).color(label_color))
        .on_press(Message::TogglePageLayout(tab_id))
        .padding([4, 10])
        .style(control_button_style);

    let continuous_label = if tab.is_continuous { "Continuous" } else { "Paginated" };
    let continuous_btn = button(text(continuous_label).size(12).color(label_color))
        .on_press(Message::ToggleContinuous(tab_id))
        .padding([4, 10])
        .style(control_button_style);

    let side_panel_label = if tab.is_side_panel_open { "Panel [x]" } else { "Panel [|]" };
    let side_panel_btn = button(text(side_panel_label).size(12).color(label_color))
        .on_press(Message::ToggleSidePanel(tab_id))
        .padding([4, 10])
        .style(control_button_style);

    let theme_text = match theme_mode {
        ThemeMode::Light => "☀️ Light",
        ThemeMode::Dark => "🌓 Dark",
    };
    let theme_btn = button(text(theme_text).size(12).color(label_color))
        .on_press(Message::ToggleTheme)
        .padding([4, 10])
        .style(control_button_style);

    let night_text = if is_night_mode { "🌙 Night: ON" } else { "☀️ Night: OFF" };
    let night_btn = button(text(night_text).size(12).color(if is_night_mode { Color::from_rgb(0.48, 0.72, 0.98) } else { label_color }))
        .on_press(Message::ToggleNightMode)
        .padding([4, 10])
        .style(move |theme, status| {
            if is_night_mode {
                button::Style {
                    background: Some(Color::from_rgb(0.20, 0.28, 0.42).into()),
                    text_color: Color::from_rgb(0.48, 0.72, 0.98),
                    border: iced::Border {
                        color: Color::from_rgb(0.38, 0.58, 0.92),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            } else {
                control_button_style(theme, status)
            }
        });

    let mut toolbar = row![
        side_panel_btn,
        prev_btn,
        page_indicator,
        next_btn,
        zoom_out_btn,
        zoom_pct,
        zoom_in_btn,
        layout_btn,
        continuous_btn,
        night_btn,
        theme_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if !tab.selected_text.is_empty() {
        let copy_btn = button(text("Copy Text").size(12).color(Color::from_rgb(0.48, 0.72, 0.98)))
            .on_press(Message::CopySelectedText)
            .padding([4, 10])
            .style(|_theme, _status| button::Style {
                background: Some(Color::from_rgb(0.18, 0.28, 0.45).into()),
                text_color: Color::from_rgb(0.48, 0.72, 0.98),
                border: iced::Border {
                    color: Color::from_rgb(0.38, 0.58, 0.92),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });
        toolbar = toolbar.push(copy_btn);
    }

    container(toolbar)
        .padding([4, 12])
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.14, 0.15, 0.18).into()),
            border: iced::Border {
                color: Color::from_rgb(0.24, 0.26, 0.32),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}