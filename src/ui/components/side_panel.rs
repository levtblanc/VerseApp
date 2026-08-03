use iced::widget::{button, column, container, image, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::messages::Message;
use crate::engine::traits::TocItem;
use crate::models::session::SidePanelTab;
use crate::models::workspace::RuntimeTab;

pub fn render_side_panel<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let tab_id = tab.id;

    let toc_tab_btn = button(text("📑 Outline").size(12))
        .on_press(Message::SetSidePanelTab(tab_id, SidePanelTab::TableOfContents))
        .padding([5.0, 10.0])
        .style(move |_theme, status| {
            let active = tab.side_panel_tab == SidePanelTab::TableOfContents;
            let bg = if active {
                Color::from_rgb(0.25, 0.32, 0.45)
            } else if matches!(status, button::Status::Hovered) {
                Color::from_rgb(0.2, 0.22, 0.26)
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.9, 0.92, 0.95),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

    let thumb_tab_btn = button(text("🖼 Thumbnails").size(12))
        .on_press(Message::SetSidePanelTab(tab_id, SidePanelTab::Thumbnails))
        .padding([5.0, 10.0])
        .style(move |_theme, status| {
            let active = tab.side_panel_tab == SidePanelTab::Thumbnails;
            let bg = if active {
                Color::from_rgb(0.25, 0.32, 0.45)
            } else if matches!(status, button::Status::Hovered) {
                Color::from_rgb(0.2, 0.22, 0.26)
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.9, 0.92, 0.95),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

    let header_row = row![toc_tab_btn, thumb_tab_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    let panel_body: Element<Message> = match tab.side_panel_tab {
        SidePanelTab::TableOfContents => {
            if tab.toc.is_empty() {
                container(text("No outline / Table of Contents found.").size(12))
                    .padding(15)
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .into()
            } else {
                let mut toc_col = column![]
                    .spacing(4)
                    .width(Length::Fill)
                    .height(Length::Shrink);

                for item in &tab.toc {
                    toc_col = toc_col.push(render_toc_item(item, tab_id, 0));
                }

                scrollable(toc_col)
                    .id(scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)))
                    .height(Length::Fill)
                    .into()
            }
        }
        SidePanelTab::Thumbnails => {
            let mut thumb_col = column![]
                .spacing(12)
                .align_x(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Shrink);

            for page_idx in 0..tab.page_count {
                let is_current = page_idx == tab.current_page;

                let card_content: Element<Message> = if let Some(handle) = tab.thumbnail_cache.get(&page_idx) {
                    image(handle.clone())
                        .width(Length::Fixed(140.0))
                        .height(Length::Fixed(180.0))
                        .into()
                } else {
                    container(text(format!("Page {}", page_idx + 1)).size(11))
                        .width(Length::Fixed(140.0))
                        .height(Length::Fixed(180.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .style(|_| container::Style {
                            background: Some(Color::from_rgb(0.18, 0.19, 0.22).into()),
                            border: iced::Border {
                                color: Color::from_rgb(0.28, 0.3, 0.35),
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        })
                        .into()
                };

                let thumb_btn = button(card_content)
                    .on_press(Message::ChangePage(tab_id, page_idx))
                    .padding(2.0)
                    .style(move |_theme, status| {
                        let border_color = if is_current {
                            Color::from_rgb(0.38, 0.58, 0.92)
                        } else if matches!(status, button::Status::Hovered) {
                            Color::from_rgb(0.5, 0.55, 0.65)
                        } else {
                            Color::TRANSPARENT
                        };
                        button::Style {
                            background: Some(Color::TRANSPARENT.into()),
                            border: iced::Border {
                                color: border_color,
                                width: if is_current { 2.0 } else { 1.0 },
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        }
                    });

                let label = text(format!("Page {}", page_idx + 1)).size(11);
                thumb_col = thumb_col.push(column![thumb_btn, label].spacing(4).align_x(Alignment::Center));
            }

            scrollable(thumb_col)
                .id(scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)))
                .height(Length::Fill)
                .into()
        }
    };

    container(
        column![header_row, panel_body]
            .spacing(10)
            .padding(10)
    )
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(Color::from_rgb(0.14, 0.15, 0.17).into()),
        border: iced::Border {
            color: Color::from_rgb(0.22, 0.24, 0.28),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn render_toc_item<'a>(item: &'a TocItem, tab_id: usize, depth: usize) -> Element<'a, Message> {
    let target_page = item.page_index;
    let title_text = item.title.clone();

    let btn = button(text(title_text).size(12))
        .on_press(Message::ChangePage(tab_id, target_page))
        .padding([4.0, 8.0])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgb(0.22, 0.26, 0.34),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::from_rgb(0.9, 0.92, 0.95),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

    let mut item_col = column![
        row![]
            .width(Length::Fixed((depth * 14) as f32))
            .push(btn)
    ]
    .spacing(2)
    .height(Length::Shrink);

    for child in &item.children {
        item_col = item_col.push(render_toc_item(child, tab_id, depth + 1));
    }

    item_col.into()
}