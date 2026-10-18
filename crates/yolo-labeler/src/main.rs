mod app;
mod classes;
mod data_yaml;
mod drag;
mod image_set;
mod menu;
mod render;
mod yolo;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use std::path::PathBuf;

use crate::app::LabelerApp;

use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions,
    actions, px, size,
};

actions!(
    yolo_labeler,
    [
        NextImage,
        PrevImage,
        DeleteSelected,
        SaveLabels,
        OpenFolder,
        Deselect,
        ResetView,
    ]
);

fn main() {
    let initial_folder = std::env::args().nth(1).map(PathBuf::from);
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("right", NextImage, None),
            KeyBinding::new("left", PrevImage, None),
            KeyBinding::new("ctrl-s", SaveLabels, None),
            KeyBinding::new("ctrl-o", OpenFolder, None),
            KeyBinding::new("delete", DeleteSelected, None),
            KeyBinding::new("backspace", DeleteSelected, None),
            KeyBinding::new("escape", Deselect, None),
            KeyBinding::new("ctrl-0", ResetView, None),
        ]);
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("YOLO Labeler".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        match cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| LabelerApp::new(initial_folder, cx));
            view.read(cx).focus_handle.focus(window);
            view
        }) {
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: failed to open window: {error:#}");
                cx.quit();
            }
        }
    });
}
