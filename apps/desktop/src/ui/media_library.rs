use crate::editor::{AddClipCommand, EditorCore};
use gpui::{InteractiveElement, *};

pub struct MediaLibrary {
    core: Entity<EditorCore>,
}

impl MediaLibrary {
    pub fn new(core: Entity<EditorCore>) -> Self {
        Self { core }
    }

    pub fn render(self, cx: &mut Context<crate::ui::Workspace>) -> impl IntoElement {
        let core = self.core.clone();

        let mut unique_assets = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for track in &core.read(cx).timeline.tracks {
            for clip in &track.clips {
                if clip.clip_type.as_deref() != Some("text") {
                    let key = if clip.path.is_empty() {
                        clip.name.clone()
                    } else {
                        clip.path.clone()
                    };
                    if seen.insert(key) {
                        unique_assets.push(clip.clone());
                    }
                }
            }
        }

        let mut asset_elements = Vec::new();
        for asset in unique_assets {
            let lower_name = asset.name.to_lowercase();
            let is_audio = lower_name.ends_with(".mp3")
                || lower_name.ends_with(".wav")
                || lower_name.ends_with(".aac")
                || lower_name.ends_with(".m4a");
            let kind = if is_audio {
                "音訊 (MP3)".to_string()
            } else {
                "影片 (H.264)".to_string()
            };
            let accent = if is_audio {
                rgb(0x10b981)
            } else {
                rgb(0x8b5cf6)
            };
            let duration_str = format!("{:.1} 秒", asset.duration);

            asset_elements.push(asset_card(asset.name, kind, duration_str, accent));
        }

        div()
            .w(px(252.))
            .h_full()
            .bg(rgb(0x161618))
            .border_r(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col()
            .child(
                // Panel Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b(px(1.))
                    .border_color(rgb(0x242427))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("素材庫")
                    )
            )
            .child(
                // Action Buttons Row below header
                div()
                    .flex()
                    .px_4()
                    .py_2()
                    .gap_2()
                    .border_b(px(1.))
                    .border_color(rgb(0x242427))
                    .child(
                        // Add Asset Button
                        div()
                            .id("add-clip-btn")
                            .flex_1()
                            .px_2()
                            .py_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(0x8b5cf6))
                            .hover(|style| style.bg(rgb(0x7c3aed)))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .on_click({
                                let core = core.clone();
                                move |_, _, cx| {
                                    println!("Add Clip button clicked!");
                                    let core = core.clone();
                                    let task = cx.prompt_for_paths(gpui::PathPromptOptions {
                                        files: true,
                                        directories: false,
                                        multiple: false,
                                        prompt: Some("選擇要新增的影片或音訊檔案".into()),
                                    });

                                    cx.spawn(move |cx: &mut gpui::AsyncApp| {
                                        let mut cx = cx.clone();
                                        async move {
                                            println!("Spawned async file picker task, awaiting path...");
                                            let paths = task.await;
                                            println!("File picker returned paths: {:?}", paths);
                                            if let Ok(Ok(Some(paths))) = paths {
                                                if let Some(path) = paths.first() {
                                                    let file_name = path.file_name()
                                                        .and_then(|f| f.to_str())
                                                        .unwrap_or("新增剪輯.mp4")
                                                        .to_string();
                                                    let absolute_path = path.to_string_lossy().to_string();
                                                    println!("Selected file name: {}, path: {}", file_name, absolute_path);
                                                    let lower_name = file_name.to_lowercase();
                                                    let is_audio = lower_name.ends_with(".mp3") ||
                                                                   lower_name.ends_with(".wav") ||
                                                                   lower_name.ends_with(".aac") ||
                                                                   lower_name.ends_with(".m4a");
                                                    let track_index = if is_audio { 2 } else { 0 };
                                                    let color = if is_audio { "#10b981".to_string() } else { "#f43f5e".to_string() };

                                                    let _ = core.update(&mut cx, |core, cx| {
                                                        println!("Executing AddClipCommand for {}, track_index: {}", file_name, track_index);
                                                        core.execute_command(
                                                            Box::new(AddClipCommand::new(
                                                                file_name,
                                                                absolute_path.clone(),
                                                                15.0,
                                                                8.0,
                                                                color,
                                                                track_index,
                                                            )),
                                                            cx,
                                                        );
                                                    });
                                                }
                                            } else {
                                                println!("Paths check failed or was cancelled.");
                                            }
                                        }
                                    }).detach();
                                }
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("+ 新增剪輯")
                            )
                    )
                    .child(
                        // Add Text Button
                        div()
                            .id("add-text-btn")
                            .flex_1()
                            .px_2()
                            .py_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(0xec4899))
                            .hover(|style| style.bg(rgb(0xdb2777)))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .on_click({
                                let core = core.clone();
                                move |_, _, cx| {
                                    let core = core.clone();
                                    let current_time = core.read(cx).playback.current_time;
                                    let _ = core.update(cx, |core, cx| {
                                        core.execute_command(
                                            Box::new(crate::editor::AddTextClipCommand::new(
                                                "新增文字".to_string(),
                                                "預設文字內容".to_string(),
                                                current_time,
                                                5.0,
                                                28.0,
                                                "#ec4899".to_string(),
                                                0,
                                            )),
                                            cx,
                                        );
                                    });
                                }
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("+ 新增文字")
                            )
                    )
            )
            .child(
                // Assets List
                div()
                    .id("media-library-assets-scroll")
                    .flex_1()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .overflow_y_scroll()
                    .children(asset_elements)
            )
    }
}

fn asset_card(name: String, kind: String, duration: String, accent: Rgba) -> impl IntoElement {
    div()
        .p_3()
        .bg(rgb(0x242427))
        .rounded(px(6.))
        .border(px(1.))
        .border_color(rgb(0x2f2f33))
        .hover(|style| style.border_color(rgb(0x48484f)))
        .flex()
        .items_center()
        .gap_3()
        .child(
            // Colored tag representing media type
            div().w(px(4.)).h(px(32.)).rounded_full().bg(accent),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0xffffff))
                        .child(name),
                )
                .child(div().text_xs().text_color(rgb(0x8e8e93)).child(kind)),
        )
        .child(div().text_xs().text_color(rgb(0xc5c5c7)).child(duration))
}
