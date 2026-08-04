use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::messages::Message;
use crate::models::session::{Action, AppSettings, ThemeMode};
use crate::ui::theme::transparent_scrollable_style;

pub fn render_settings_modal<'a>(
    settings: &'a AppSettings,
    remapping_action: Option<Action>,
    base_content: Element<'a, Message>,
) -> Element<'a, Message> {
    // Theme-independent light text colors for dark card background
    let text_primary = Color::from_rgb(0.95, 0.95, 0.98);
    let text_secondary = Color::from_rgb(0.70, 0.73, 0.80);

    let header_title = text("⚙ Settings").size(20).color(text_primary);
    let header_subtitle = text("Keyboard shortcuts & application preferences").size(12).color(text_secondary);

    let close_btn = button(text("✕").size(12).color(text_primary))
        .on_press(Message::CloseSettings)
        .padding([4, 8])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.95, 0.95, 0.98),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let header_row = row![
        column![header_title, header_subtitle].spacing(2),
        container(close_btn).align_x(Alignment::End).width(Length::Fill)
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    // --- Theme Switcher Section ---
    let theme_label = text("Theme Mode").size(14).color(text_primary);
    let theme_val_label = match settings.theme {
        ThemeMode::Dark => "🌓 Dark Theme",
        ThemeMode::Light => "☀️ Light Theme",
    };
    let theme_toggle_btn = button(text(theme_val_label).size(12).color(text_primary))
        .on_press(Message::ToggleTheme)
        .padding([6, 12])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.28, 0.30, 0.36),
                _ => Color::from_rgb(0.20, 0.22, 0.26),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.95, 0.95, 0.98),
                border: iced::Border {
                    color: Color::from_rgb(0.30, 0.32, 0.38),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        });

    let theme_card = container(
        row![theme_label, container(theme_toggle_btn).align_x(Alignment::End).width(Length::Fill)]
            .align_y(Alignment::Center)
            .width(Length::Fill)
    )
    .padding(12)
    .style(|_| container::Style {
        background: Some(Color::from_rgb(0.16, 0.17, 0.20).into()),
        border: iced::Border {
            color: Color::from_rgb(0.24, 0.26, 0.30),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    // --- Keybindings List ---
    let mut keybindings_col = column![].spacing(6).width(Length::Fill);

    let key_actions = vec![
        Action::OpenFile,
        Action::NextPage,
        Action::PrevPage,
        Action::NextTab,
        Action::PrevTab,
        Action::CloseActiveTab,
        Action::ToggleSidePanel,
        Action::ToggleTabBar,
        Action::TogglePageLayout,
        Action::ToggleContinuous,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::ToggleFullscreen,
        Action::ToggleTheme,
        Action::OpenSettings,
    ];

    for action in key_actions {
        if let Some(binding) = settings.keybindings.get(&action) {
            let action_name = text(action.display_name()).size(13).color(text_primary);
            let binding_text = if remapping_action == Some(action) {
                "Press new key...".to_string()
            } else {
                binding.to_display_string()
            };

            let is_remapping = remapping_action == Some(action);

            let bind_btn = button(text(binding_text).size(12).color(text_primary))
                .on_press(Message::StartRemapping(action))
                .padding([4, 10])
                .style(move |_theme, status| {
                    let bg = if is_remapping {
                        Color::from_rgb(0.38, 0.58, 0.92)
                    } else if matches!(status, button::Status::Hovered) {
                        Color::from_rgb(0.28, 0.30, 0.36)
                    } else {
                        Color::from_rgb(0.20, 0.22, 0.26)
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: Color::from_rgb(0.95, 0.95, 0.98),
                        border: iced::Border {
                            color: Color::from_rgb(0.30, 0.32, 0.38),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                });

            let item_row = container(
                row![action_name, container(bind_btn).align_x(Alignment::End).width(Length::Fill)]
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
            )
            .padding([8, 12])
            .style(|_| container::Style {
                background: Some(Color::from_rgb(0.16, 0.17, 0.20).into()),
                border: iced::Border {
                    color: Color::from_rgb(0.22, 0.24, 0.28),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

            keybindings_col = keybindings_col.push(item_row);
        }
    }

    let scrollable_keymaps = scrollable(keybindings_col)
        .style(transparent_scrollable_style)
        .height(Length::Fixed(340.0));

    let modal_card = container(
        column![
            header_row,
            theme_card,
            text("Keyboard Shortcuts").size(14).color(text_primary),
            scrollable_keymaps
        ]
        .spacing(14)
        .align_x(Alignment::Center)
    )
    .width(Length::Fixed(520.0))
    .padding(24)
    .style(|_| container::Style {
        background: Some(Color::from_rgb(0.12, 0.13, 0.15).into()),
        border: iced::Border {
            color: Color::from_rgb(0.26, 0.28, 0.34),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    iced::widget::stack![
        base_content,
        iced::widget::opaque(
            container(modal_card)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.65).into()),
                    ..Default::default()
                })
        )
    ]
    .into()
}