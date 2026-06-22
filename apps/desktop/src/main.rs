mod editor;
mod ui;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, px, size, prelude::*};
use editor::EditorCore;
use ui::Workspace;

gpui::actions!(opencut, [
    Undo, Redo, Split, OpenProject, SaveProject, ExportVideo,
    ApplyGrayscaleFilter, ApplyBrightenFilter, ApplyContrastFilter, ApplyNoneFilter,
    ApplyFadeTransition, ApplyNoneTransition,
    NewProject, DeleteClip, ImportClip,
    TogglePlay, SeekForward, SeekBackward, FrameStepForward, FrameStepBackward,
    ToggleSnapping
]);

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            gpui::KeyBinding::new("ctrl-z", crate::Undo, None),
            gpui::KeyBinding::new("ctrl-shift-z", crate::Redo, None),
            gpui::KeyBinding::new("ctrl-y", crate::Redo, None),
            gpui::KeyBinding::new("ctrl-k", crate::Split, None),
            gpui::KeyBinding::new("s", crate::Split, None),
            gpui::KeyBinding::new("n", crate::ToggleSnapping, None),
            gpui::KeyBinding::new("ctrl-o", crate::OpenProject, None),
            gpui::KeyBinding::new("ctrl-s", crate::SaveProject, None),
            gpui::KeyBinding::new("ctrl-e", crate::ExportVideo, None),
            gpui::KeyBinding::new("ctrl-n", crate::NewProject, None),
            gpui::KeyBinding::new("delete", crate::DeleteClip, None),
            gpui::KeyBinding::new("backspace", crate::DeleteClip, None),
            gpui::KeyBinding::new("ctrl-i", crate::ImportClip, None),
            gpui::KeyBinding::new("space", crate::TogglePlay, None),
            gpui::KeyBinding::new("k", crate::TogglePlay, None),
            gpui::KeyBinding::new("l", crate::SeekForward, None),
            gpui::KeyBinding::new("j", crate::SeekBackward, None),
            gpui::KeyBinding::new("right", crate::FrameStepForward, None),
            gpui::KeyBinding::new("left", crate::FrameStepBackward, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                // In GPUI 0.2.2, cx.new() is used to create entities (both views and models).
                // It returns an Entity<T> handle.
                let core = cx.new(|_| EditorCore::new());
                cx.new(|cx| Workspace::new(core, cx))
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
