use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::ui::components::settings::render_settings_modal;
use crate::ui::components::tab_bar::render_tab_bar;
use crate::ui::components::viewer::render_document_viewer;

impl ReaderApp {
    pub fn view(&self) -> Element<Message> {
        let active_id = self.active_tab_id.unwrap_or(0);

        let settings_btn = button(text("⚙").size(16))
            .on_press(Message::OpenSettings)
            .padding([6.0, 12.0])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => iced::Color::from_rgb(0.28, 0.3, 0.35),
                    _ => iced::Color::from_rgb(0.18, 0.19, 0.22),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: iced::Color::from_rgb(0.9, 0.92, 0.95),
                    border: iced::Border {
                        color: iced::Color::from_rgb(0.3, 0.32, 0.38),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            });

        let primary_view: Element<Message> = if self.tabs.is_empty() {
            container(
                column![
                    text("📚 Multi-Format Document Reader").size(24),
                    text("Support for PDF, EPUB, DOCX, DJVU, XPS, MOBI, CBZ, FB2").size(14),
                    button(text("📂 Open Document (Ctrl + O)"))
                        .on_press(Message::OpenFileRequested)
                        .padding([10.0, 20.0])
                ]
                .spacing(15)
                .align_x(iced::Alignment::Center)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
            render_document_viewer(tab)
        } else {
            container(text("Select a Tab").size(18))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let body_content: Element<Message> = if let Some(sec_id) = self.split_secondary_tab_id {
            if let Some(sec_tab) = self.tabs.iter().find(|t| t.id == sec_id) {
                row![
                    primary_view,
                    render_document_viewer(sec_tab)
                ]
                .spacing(10)
                .into()
            } else {
                primary_view
            }
        } else {
            primary_view
        };

        let content_with_error: Element<Message> = if let Some(ref err) = self.error_message {
            let error_banner = container(
                row![
                    text(format!("⚠️ {}", err)).size(13),
                    button(text("✕").size(12)).on_press(Message::ClearError)
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
            )
            .padding([6.0, 12.0])
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb(0.6, 0.15, 0.15).into()),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            column![error_banner, body_content].spacing(8).into()
        } else {
            body_content
        };

        let base_layout: Element<Message> = if self.is_tab_bar_visible {
            let top_header = row![
                render_tab_bar(&self.tabs, active_id, self.dragged_tab_id),
                settings_btn
            ]
            .spacing(8)
            .padding(iced::Padding {
                top: 8.0,
                right: 10.0,
                bottom: 2.0,
                left: 10.0,
            })
            .align_y(iced::Alignment::Center);

            column![top_header, content_with_error].into()
        } else {
            column![content_with_error].into()
        };

        // Persistent stack root guarantees base_layout never unmounts
        if self.is_settings_open {
            iced::widget::stack![
                base_layout,
                iced::widget::opaque(render_settings_modal(
                    &self.settings,
                    self.remapping_action,
                    self.active_modifiers
                ))
            ]
            .into()
        } else {
            iced::widget::stack![base_layout].into()
        }
    }
}