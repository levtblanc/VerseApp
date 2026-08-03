use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::messages::Message;
use crate::models::session::{Action, AppSettings};

pub fn render_settings_modal<'a>(
    settings: &'a AppSettings,
    remapping_action: Option<Action>,
    base_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let title = text("⚙ Settings & Keybindings").size(20);

    let mut keybindings_col = column![].spacing(8);

    for (action, binding) in &settings.keybindings {
        let action_name = text(action.display_name()).size(13);
        let binding_text = if remapping_action == Some(*action) {
            "Press new key...".to_string()
        } else {
            binding.to_display_string()
        };

        let bind_btn = button(text(binding_text).size(12))
            .on_press(Message::StartRemapping(*action))
            .padding([4, 10]);

        let row_item = row![action_name, bind_btn]
            .spacing(15)
            .align_y(Alignment::Center);

        keybindings_col = keybindings_col.push(row_item);
    }

    let close_btn = button(text("Close")).on_press(Message::CloseSettings);

    let modal_card = container(
        column![title, keybindings_col, close_btn]
            .spacing(15)
            .align_x(Alignment::Center)
    )
    .width(Length::Fixed(400.0))
    .padding(20)
    .style(|_| container::Style {
        background: Some(Color::from_rgb(0.15, 0.16, 0.18).into()),
        border: iced::Border {
            color: Color::from_rgb(0.3, 0.35, 0.4),
            width: 1.0,
            radius: 8.0.into(),
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
        )
    ]
    .into()
}