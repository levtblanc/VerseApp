mod app;
mod engine;
mod models;
mod ui;

use app::ReaderApp;

fn main() -> iced::Result {
    iced::application("Verse", ReaderApp::update, ReaderApp::view)
        .theme(ReaderApp::theme)
        .subscription(ReaderApp::subscription)
        .run_with(ReaderApp::new)
}