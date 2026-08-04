use iced::widget::{button, column, container, image, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::messages::Message;
use crate::models::session::{PageLayout, SidePanelTab};
use crate::models::workspace::RuntimeTab;
use crate::ui::components::side_panel::render_side_panel;
use crate::ui::theme::transparent_scrollable_style;

fn control_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
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

pub fn render_document_viewer<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let tab_id = tab.id;
    let label_color = Color::from_rgb(0.90, 0.92, 0.96);

    // --- Floating Control Toolbar ---
    let prev_btn = button(text("<").size(13).color(label_color))
        .on_press(Message::ChangePage(tab_id, tab.current_page.saturating_sub(1)))
        .padding([4, 10])
        .style(control_button_style);

    let page_indicator = text(format!("{} / {}", tab.current_page + 1, tab.page_count.max(1)))
        .size(13)
        .color(label_color);

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

    let theme_btn = button(text("🌓 Theme").size(12).color(label_color))
        .on_press(Message::ToggleTheme)
        .padding([4, 10])
        .style(control_button_style);

    let toolbar = row![
        side_panel_btn,
        prev_btn,
        page_indicator,
        next_btn,
        zoom_out_btn,
        zoom_pct,
        zoom_in_btn,
        layout_btn,
        continuous_btn,
        theme_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let control_tray = container(toolbar)
        .padding([4, 12])
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.14, 0.15, 0.18).into()),
            border: iced::Border {
                color: Color::from_rgb(0.24, 0.26, 0.32),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

    let v_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);
    let h_scrollbar = scrollable::Scrollbar::default().scroller_width(12.0);

    // --- Main Document Canvas ---
    let canvas_area: Element<Message> = if tab.page_count == 0 {
        container(text("Empty Document").size(16))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else if tab.is_continuous {
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
            // --- SINGLE CONTINUOUS MODE ---
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
    } else {
        // --- PAGINATED MODE (SINGLE / DOUBLE) ---
        let (approx_width, page_view): (f32, Element<Message>) = match tab.layout {
            PageLayout::Single => {
                let (doc_w, doc_h) = tab.backend.page_dimensions(tab.current_page);
                let target_w = doc_w * tab.zoom;
                let target_h = doc_h * tab.zoom;

                let view: Element<Message> = if let Some(handle) = tab.get_texture(tab.current_page) {
                    image(handle.clone())
                        .width(Length::Fixed(target_w))
                        .height(Length::Fixed(target_h))
                        .into()
                } else {
                    container(text(format!("Loading Page {}...", tab.current_page + 1)).size(13))
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
                    image(handle.clone())
                        .width(Length::Fixed(left_target_w))
                        .height(Length::Fixed(left_target_h))
                        .into()
                } else {
                    container(text(format!("Page {}", left_page + 1)).size(12))
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
                        image(handle.clone())
                            .width(Length::Fixed(rw))
                            .height(Length::Fixed(rh))
                            .into()
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

        let (dir, container_w) = if approx_width > 1300.0 {
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
            .width(Length::Fill)
            .height(Length::Fill);

        container(scrollable_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    };
    
    let main_workspace = column![
        control_tray,
        canvas_area
    ]
    .spacing(8)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    if tab.is_side_panel_open {
        row![
            render_side_panel(tab),
            main_workspace
        ]
        .spacing(12)
        .padding(iced::Padding {
            top: 4.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        })
        .into()
    } else {
        container(main_workspace)
            .padding(iced::Padding {
                top: 4.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            })
            .into()
    }
}