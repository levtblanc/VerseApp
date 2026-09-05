use iced::widget::{button, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length};

use crate::app::messages::Message;
use crate::models::session::ThemeMode;
use crate::models::workspace::RuntimeTab;
use crate::ui::theme::invisible_scrollable_style;

pub fn render_tab_bar<'a>(
    tabs: &'a [RuntimeTab],
    active_id: usize,
    dragged_tab_id: Option<usize>,
    theme_mode: ThemeMode,
) -> Element<'a, Message> {
    let is_dark = matches!(theme_mode, ThemeMode::Dark);

    // Color definitions
    let active_bg = if is_dark {
        Color::from_rgb(0.18, 0.20, 0.24)
    } else {
        Color::from_rgb(0.98, 0.98, 1.0)
    };

    let inactive_bg = Color::TRANSPARENT;
    let inactive_hover_bg = if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.05)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.05)
    };

    let active_text_color = if is_dark {
        Color::from_rgb(0.95, 0.96, 0.98)
    } else {
        Color::from_rgb(0.12, 0.14, 0.18)
    };

    let inactive_text_color = if is_dark {
        Color::from_rgb(0.60, 0.64, 0.72)
    } else {
        Color::from_rgb(0.40, 0.44, 0.52)
    };

    let divider_color = if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.12)
    };

    let mut tab_strip = row![].spacing(2).align_y(Alignment::Center);

    for (idx, tab) in tabs.iter().enumerate() {
        let is_active = tab.id == active_id;
        let is_dragging = dragged_tab_id == Some(tab.id);
        let tab_id = tab.id;

        // 1. File Format Miniature "Favicon" Badge
        let ext = tab.file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("DOC")
            .to_uppercase();
        let badge_text = if ext.len() > 4 { &ext[..4] } else { &ext };

        let (badge_bg, badge_fg) = match ext.as_str() {
            "PDF" => (Color::from_rgb(0.70, 0.20, 0.20), Color::WHITE),
            "EPUB" | "MOBI" | "FB2" => (Color::from_rgb(0.20, 0.55, 0.35), Color::WHITE),
            "DOCX" => (Color::from_rgb(0.18, 0.40, 0.75), Color::WHITE),
            "DJVU" => (Color::from_rgb(0.75, 0.45, 0.15), Color::WHITE),
            "CBZ" | "CBR" => (Color::from_rgb(0.55, 0.25, 0.65), Color::WHITE),
            _ => (Color::from_rgb(0.35, 0.38, 0.45), Color::WHITE),
        };

        let badge = container(
            text(badge_text.to_string())
                .size(9)
                .color(badge_fg)
        )
        .padding([2.0, 4.0])
        .style(move |_| container::Style {
            background: Some(badge_bg.into()),
            border: Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        });

        // 2. Truncated Title Label
        let display_title = if tab.title.len() > 22 {
            format!("{}…", &tab.title[..20])
        } else {
            tab.title.clone()
        };

        let title_label = text(display_title)
            .size(12)
            .color(if is_active { active_text_color } else { inactive_text_color });

        // 3. Circular Close Button
        let close_btn = button(
            container(text("✕").size(9)).center_x(Length::Fill).center_y(Length::Fill)
        )
        .on_press(Message::CloseTab(tab_id))
        .padding(0)
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if matches!(status, button::Status::Hovered) {
                    Color::WHITE
                } else if is_active {
                    active_text_color
                } else {
                    inactive_text_color
                },
                border: Border { radius: 9.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

        // 4. Tab Card Assembly
        let tab_content = row![
            badge,
            container(title_label).width(Length::Fill),
            close_btn
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let tab_card = container(tab_content)
            .width(Length::Fixed(190.0))
            .height(Length::Fixed(34.0))
            .padding([0.0, 10.0])
            .align_y(Alignment::Center)
            .style(move |_theme| {
                let bg = if is_dragging {
                    if is_dark { Color::from_rgb(0.24, 0.28, 0.36) } else { Color::from_rgb(0.85, 0.90, 0.98) }
                } else if is_active {
                    active_bg
                } else {
                    inactive_bg
                };

                let border = if is_dragging {
                    Border {
                        color: Color::from_rgb(0.38, 0.58, 0.92),
                        width: 1.5,
                        radius: 8.0.into(),
                    }
                } else if is_active {
                    Border {
                        color: if is_dark { Color::from_rgba(1.0, 1.0, 1.0, 0.08) } else { Color::from_rgba(0.0, 0.0, 0.0, 0.08) },
                        width: 1.0,
                        radius: 8.0.into(),
                    }
                } else {
                    Border { radius: 8.0.into(), ..Default::default() }
                };

                container::Style {
                    background: Some(bg.into()),
                    border,
                    ..Default::default()
                }
            });

        // Interactive mouse wrapper for drag-and-drop & tab switching
        let tab_interactive = mouse_area(tab_card)
            .on_press(Message::SelectTab(tab_id))
            .on_enter(Message::TabDraggedOver(tab_id));

        tab_strip = tab_strip.push(tab_interactive);

        // Subtle divider between inactive tabs
        let next_is_active = tabs.get(idx + 1).map(|t| t.id == active_id).unwrap_or(false);
        if !is_active && !next_is_active && idx < tabs.len() - 1 {
            let divider = container(row![])
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(16.0))
                .style(move |_| container::Style {
                    background: Some(divider_color.into()),
                    ..Default::default()
                });
            tab_strip = tab_strip.push(divider);
        }
    }

    // Modern '+' New Tab Button
    let new_tab_btn = button(
        container(text("+").size(15)).center_x(Length::Fill).center_y(Length::Fill)
    )
    .on_press(Message::OpenFileRequested)
    .padding(0)
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .style(move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => {
                if is_dark { Color::from_rgba(1.0, 1.0, 1.0, 0.08) } else { Color::from_rgba(0.0, 0.0, 0.0, 0.06) }
            }
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: if is_dark { Color::from_rgb(0.75, 0.78, 0.85) } else { Color::from_rgb(0.30, 0.34, 0.40) },
            border: Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        }
    });

    tab_strip = tab_strip.push(new_tab_btn);

    // Smooth invisible horizontal scrollbar
    let scrollable_strip = scrollable(tab_strip)
        .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
        .style(invisible_scrollable_style)
        .height(Length::Shrink);

    container(scrollable_strip)
        .width(Length::Fill)
        .padding([4.0, 6.0])
        .style(move |_| container::Style {
            background: Some(if is_dark {
                Color::from_rgb(0.11, 0.12, 0.14).into()
            } else {
                Color::from_rgb(0.88, 0.90, 0.94).into()
            }),
            border: Border {
                color: if is_dark { Color::from_rgba(1.0, 1.0, 1.0, 0.05) } else { Color::from_rgba(0.0, 0.0, 0.0, 0.06) },
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}
