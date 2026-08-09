pub mod continuous;
pub mod page_canvas;
pub mod paginated;
pub mod search_bar;
pub mod toolbar;

use iced::widget::{column, container, row, stack};
use iced::{Alignment, Element, Length};

use crate::app::messages::Message;
use crate::models::session::ThemeMode;
use crate::models::workspace::RuntimeTab;
use crate::ui::components::side_panel::render_side_panel;
use crate::ui::components::viewer::continuous::render_continuous_view;
use crate::ui::components::viewer::paginated::render_paginated_view;
use crate::ui::components::viewer::search_bar::render_search_bar;
use crate::ui::components::viewer::toolbar::render_control_tray;

pub fn render_document_viewer<'a>(
    tab: &'a RuntimeTab,
    theme_mode: ThemeMode,
    is_night_mode: bool,
) -> Element<'a, Message> {
    let control_tray = render_control_tray(tab, theme_mode, is_night_mode);

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

    let floating_search: Element<Message> = if tab.is_search_open {
        container(render_search_bar(tab))
            .padding(iced::Padding {
                top: 50.0,
                right: 20.0,
                bottom: 0.0,
                left: 0.0,
            })
            .align_x(Alignment::End)
            .into()
    } else {
        container(row![]).width(Length::Shrink).height(Length::Shrink).into()
    };

    let main_workspace_with_search = stack![main_workspace, floating_search];

    if tab.is_side_panel_open {
        row![
            render_side_panel(tab),
            main_workspace_with_search
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
        container(main_workspace_with_search)
            .padding(iced::Padding {
                top: 4.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            })
            .into()
    }
}