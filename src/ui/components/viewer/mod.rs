pub mod continuous;
pub mod paginated;
pub mod toolbar;

use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};

use crate::app::messages::Message;
use crate::models::workspace::RuntimeTab;
use crate::ui::components::side_panel::render_side_panel;
use crate::ui::components::viewer::continuous::render_continuous_view;
use crate::ui::components::viewer::paginated::render_paginated_view;
use crate::ui::components::viewer::toolbar::render_control_tray;

pub fn render_document_viewer<'a>(tab: &'a RuntimeTab) -> Element<'a, Message> {
    let control_tray = render_control_tray(tab);

    let canvas_area: Element<Message> = if tab.page_count == 0 {
        container(iced::widget::text("Empty Document").size(16))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else if tab.is_continuous {
        render_continuous_view(tab)
    } else {
        render_paginated_view(tab)
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