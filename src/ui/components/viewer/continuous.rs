use iced::widget::{column, container, image, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};

use crate::app::messages::Message;
use crate::models::session::PageLayout;
use crate::models::workspace::RuntimeTab;
use crate::ui::theme::transparent_scrollable_style;

pub fn render_continuous_view<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let tab_id = tab.id;
    let v_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);
    let h_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);

    if tab.layout == PageLayout::Double {
        let mut rows_col = column![]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Shrink)
            .height(Length::Shrink);

        let total_rows = (tab.page_count + 1) / 2;
        let active_row = tab.current_page / 2;
        let visible_row_start = active_row.saturating_sub(4);
        let visible_row_end = (active_row + 8).min(total_rows);

        let (sample_w, _) = tab.backend.page_dimensions(0);
        let approx_pair_width = (sample_w * 2.0 + 8.0) * tab.zoom;

        for row_idx in 0..total_rows {
            let left_page = row_idx * 2;
            let right_page = left_page + 1;
            let is_in_viewport = row_idx >= visible_row_start && row_idx < visible_row_end;

            let (left_w_raw, left_h_raw) = tab.backend.page_dimensions(left_page);
            let left_target_w = left_w_raw * tab.zoom;
            let left_target_h = left_h_raw * tab.zoom;

            if is_in_viewport {
                let left_view: Element<Message> = if let Some(handle) = tab.get_texture(left_page) {
                    image(handle.clone())
                        .width(Length::Fixed(left_target_w))
                        .height(Length::Fixed(left_target_h))
                        .into()
                } else {
                    container(text(format!("Loading Page {}...", left_page + 1)).size(12))
                        .width(Length::Fixed(left_target_w))
                        .height(Length::Fixed(left_target_h))
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

                let right_view: Element<Message> = if right_page < tab.page_count {
                    let (right_w_raw, right_h_raw) = tab.backend.page_dimensions(right_page);
                    let right_target_w = right_w_raw * tab.zoom;
                    let right_target_h = right_h_raw * tab.zoom;

                    if let Some(handle) = tab.get_texture(right_page) {
                        image(handle.clone())
                            .width(Length::Fixed(right_target_w))
                            .height(Length::Fixed(right_target_h))
                            .into()
                    } else {
                        container(text(format!("Loading Page {}...", right_page + 1)).size(12))
                            .width(Length::Fixed(right_target_w))
                            .height(Length::Fixed(right_target_h))
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
                    }
                } else {
                    container(row![])
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                };

                let double_row_widget = row![left_view, right_view]
                    .spacing(4)
                    .align_y(Alignment::Center);

                rows_col = rows_col.push(double_row_widget);
            } else {
                let right_target_w = if right_page < tab.page_count {
                    let (rw, _) = tab.backend.page_dimensions(right_page);
                    rw * tab.zoom
                } else {
                    0.0
                };
                let row_w = left_target_w + right_target_w + (if right_page < tab.page_count { 4.0 } else { 0.0 });
                let row_h = left_target_h;

                let blank_box = container(row![])
                    .width(Length::Fixed(row_w))
                    .height(Length::Fixed(row_h));
                rows_col = rows_col.push(blank_box);
            }
        }

        let (dir, container_w) = if approx_pair_width > 1300.0 {
            (
                scrollable::Direction::Both {
                    vertical: v_scrollbar,
                    horizontal: h_scrollbar,
                },
                Length::Shrink,
            )
        } else {
            (
                scrollable::Direction::Vertical(v_scrollbar),
                Length::Fill,
            )
        };

        let inner_container = container(rows_col)
            .width(container_w)
            .align_x(Alignment::Center);

        let scrollable_view = scrollable(inner_container)
            .direction(dir)
            .style(transparent_scrollable_style)
            .id(scrollable::Id::new(format!("viewer_scroll_{}", tab_id)))
            .on_scroll(move |viewport| Message::ViewportScrolled {
                tab_id,
                offset_y: viewport.absolute_offset().y,
            })
            .width(Length::Fill)
            .height(Length::Fill);

        container(scrollable_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    } else {
        let mut pages_col = column![]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Shrink)
            .height(Length::Shrink);

        let visible_start = tab.current_page.saturating_sub(4);
        let visible_end = (tab.current_page + 9).min(tab.page_count);

        let (sample_w, _) = tab.backend.page_dimensions(0);
        let approx_page_width = sample_w * tab.zoom;

        for page_idx in 0..tab.page_count {
            let is_in_viewport = page_idx >= visible_start && page_idx < visible_end;
            let (doc_w, doc_h) = tab.backend.page_dimensions(page_idx);
            let target_w = doc_w * tab.zoom;
            let target_h = doc_h * tab.zoom;

            if is_in_viewport {
                if let Some(handle) = tab.get_texture(page_idx) {
                    let img_widget = image(handle.clone())
                        .width(Length::Fixed(target_w))
                        .height(Length::Fixed(target_h));
                    pages_col = pages_col.push(img_widget);
                } else {
                    let card = container(text(format!("Loading Page {}...", page_idx + 1)).size(12))
                        .width(Length::Fixed(target_w))
                        .height(Length::Fixed(target_h))
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
                        });
                    pages_col = pages_col.push(card);
                }
            } else {
                let blank_box = container(row![])
                    .width(Length::Fixed(target_w))
                    .height(Length::Fixed(target_h));
                pages_col = pages_col.push(blank_box);
            }
        }

        let (dir, container_w) = if approx_page_width > 1300.0 {
            (
                scrollable::Direction::Both {
                    vertical: v_scrollbar,
                    horizontal: h_scrollbar,
                },
                Length::Shrink,
            )
        } else {
            (
                scrollable::Direction::Vertical(v_scrollbar),
                Length::Fill,
            )
        };

        let inner_container = container(pages_col)
            .width(container_w)
            .align_x(Alignment::Center);

        let scrollable_view = scrollable(inner_container)
            .direction(dir)
            .style(transparent_scrollable_style)
            .id(scrollable::Id::new(format!("viewer_scroll_{}", tab_id)))
            .on_scroll(move |viewport| Message::ViewportScrolled {
                tab_id,
                offset_y: viewport.absolute_offset().y,
            })
            .width(Length::Fill)
            .height(Length::Fill);

        container(scrollable_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    }
}