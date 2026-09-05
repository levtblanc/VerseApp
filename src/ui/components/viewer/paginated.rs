use iced::widget::canvas::Canvas;
use iced::widget::{container, image, row, scrollable, stack, text};
use iced::{Alignment, Color, Element, Length};

use crate::app::messages::Message;
use crate::models::session::PageLayout;
use crate::models::workspace::RuntimeTab;
use crate::ui::components::viewer::page_canvas::PageSelectionProgram;
use crate::ui::theme::transparent_scrollable_style;

pub fn render_paginated_view<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let tab_id = tab.id;
    let v_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);
    let h_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);

    let (approx_width, page_view): (f32, Element<Message>) = match tab.layout {
        PageLayout::Single => {
            let page_idx = tab.current_page;
            let (doc_w, doc_h) = tab.backend.page_dimensions(page_idx);
            let target_w = doc_w * tab.zoom;
            let target_h = doc_h * tab.zoom;

            let view: Element<Message> = if let Some(handle) = tab.get_texture(page_idx) {
                let img_widget = image(handle.clone())
                    .width(Length::Fixed(target_w))
                    .height(Length::Fixed(target_h));

                let selected_quads = tab.get_selected_quads_for_page(page_idx);
                let search_quads = tab.get_search_matches_for_page(page_idx);
                let active_search_quad = tab.get_active_search_match_for_page(page_idx);

                let selection_canvas = Canvas::new(PageSelectionProgram {
                    page_index: page_idx,
                    zoom: tab.zoom,
                    selected_quads,
                    search_quads,
                    active_search_quad,
                })
                .width(Length::Fixed(target_w))
                .height(Length::Fixed(target_h));

                stack![img_widget, selection_canvas].into()
            } else {
                container(text(format!("Loading Page {}...", page_idx + 1)).size(13))
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
                    })
                    .into()
            };

            (target_w, view)
        }
        PageLayout::Double => {
            let left_page = tab.current_page;
            let right_page = tab.current_page + 1;

            let (left_w_raw, left_h_raw) = tab.backend.page_dimensions(left_page);
            let left_target_w = left_w_raw * tab.zoom;
            let left_target_h = left_h_raw * tab.zoom;

            let left_view: Element<Message> = if let Some(handle) = tab.get_texture(left_page) {
                let img_widget = image(handle.clone())
                    .width(Length::Fixed(left_target_w))
                    .height(Length::Fixed(left_target_h));

                let selected_quads = tab.get_selected_quads_for_page(left_page);
                let search_quads = tab.get_search_matches_for_page(left_page);
                let active_search_quad = tab.get_active_search_match_for_page(left_page);

                let selection_canvas = Canvas::new(PageSelectionProgram {
                    page_index: left_page,
                    zoom: tab.zoom,
                    selected_quads,
                    search_quads,
                    active_search_quad,
                })
                .width(Length::Fixed(left_target_w))
                .height(Length::Fixed(left_target_h));

                stack![img_widget, selection_canvas].into()
            } else {
                container(text(format!("Loading Page {}...", left_page + 1)).size(12))
                    .width(Length::Fixed(left_target_w))
                    .height(Length::Fixed(left_target_h))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(|_| container::Style {
                        background: Some(Color::from_rgb(0.18, 0.19, 0.22).into()),
                        ..Default::default()
                    })
                    .into()
            };

            let (right_target_w, right_view): (f32, Element<Message>) = if right_page < tab.page_count {
                let (right_w_raw, right_h_raw) = tab.backend.page_dimensions(right_page);
                let rw = right_w_raw * tab.zoom;
                let rh = right_h_raw * tab.zoom;

                let r_elem: Element<Message> = if let Some(handle) = tab.get_texture(right_page) {
                    let img_widget = image(handle.clone())
                        .width(Length::Fixed(rw))
                        .height(Length::Fixed(rh));

                    let selected_quads = tab.get_selected_quads_for_page(right_page);
                    let search_quads = tab.get_search_matches_for_page(right_page);
                    let active_search_quad = tab.get_active_search_match_for_page(right_page);

                    let selection_canvas = Canvas::new(PageSelectionProgram {
                        page_index: right_page,
                        zoom: tab.zoom,
                        selected_quads,
                        search_quads,
                        active_search_quad,
                    })
                    .width(Length::Fixed(rw))
                    .height(Length::Fixed(rh));

                    stack![img_widget, selection_canvas].into()
                } else {
                    container(text(format!("Page {}", right_page + 1)).size(12))
                        .width(Length::Fixed(rw))
                        .height(Length::Fixed(rh))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .style(|_| container::Style {
                            background: Some(Color::from_rgb(0.18, 0.19, 0.22).into()),
                            ..Default::default()
                        })
                        .into()
                };
                (rw, r_elem)
            } else {
                (0.0, container(row![]).width(Length::Shrink).height(Length::Shrink).into())
            };

            let total_w = left_target_w + right_target_w + 4.0;
            let double_row = row![left_view, right_view]
                .spacing(4)
                .align_y(Alignment::Center)
                .into();

            (total_w, double_row)
        }
    };

    let needs_horizontal = approx_width > tab.viewport_width;
    let (dir, container_w) = if needs_horizontal {
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

    let inner_container = container(page_view)
        .width(container_w)
        .align_x(Alignment::Center);

    let scrollable_view = scrollable(inner_container)
        .direction(dir)
        .style(transparent_scrollable_style)
        .id(scrollable::Id::new(format!("viewer_scroll_{}", tab_id)))
        .on_scroll(move |viewport| {
            let y = viewport.absolute_offset().y;
            let safe_y = if y.is_finite() { y.max(0.0) } else { 0.0 };
            Message::ViewportScrolled {
                tab_id,
                offset_y: safe_y,
                viewport_width: viewport.bounds().width,
                viewport_height: viewport.bounds().height,
            }
        })
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrollable_view)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .into()
}
