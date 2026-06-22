use gpui::{*, InteractiveElement};
use crate::editor::EditorCore;

pub struct Timeline {
    core: Entity<EditorCore>,
    selected_clip_id: Option<String>,
    snapped_time: Option<f64>,
}

impl Timeline {
    pub fn new(core: Entity<EditorCore>, selected_clip_id: Option<String>, snapped_time: Option<f64>) -> Self {
        Self { core, selected_clip_id, snapped_time }
    }

    pub fn render(self, cx: &mut Context<crate::ui::Workspace>) -> impl IntoElement {
        let core = self.core.clone();
        let current_time = core.read(cx).playback.current_time;
        let duration = core.read(cx).playback.duration;
        let tracks = core.read(cx).timeline.tracks.clone();

        let snapped_time = self.snapped_time;

        // Trigger audio waveform extraction for all clips upfront
        let mut paths = Vec::new();
        for track in &core.read(cx).timeline.tracks {
            for clip in &track.clips {
                paths.push(clip.path.clone());
            }
        }

        let _ = core.update(cx, |core, cx| {
            for path in paths {
                core.load_audio_waveform(path.clone(), cx);
                core.load_audio_samples(path, cx);
            }
        });
        
        let audio_waveforms = core.read(cx).audio_waveforms.clone();

        let progress_ratio = (current_time / duration).clamp(0.0, 1.0);

        // Generate ticks (ruler markings) every 5 seconds (0s, 5s, 10s, ..., 60s)
        let mut ticks = Vec::new();
        let num_ticks = 13;
        for i in 0..num_ticks {
            let tick_time = (i as f64) * 5.0;
            let tick_ratio = tick_time / duration;
            let core = core.clone();

            ticks.push(
                div()
                    .id(("tick", i as usize))
                    .absolute()
                    .left(relative(tick_ratio as f32))
                    .flex()
                    .flex_col()
                    .items_center()
                    .on_click(move |_, _, cx| {
                        let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                            core.seek(tick_time, cx);
                        });
                    })
                    .child(
                        // Little tick mark line
                        div().w(px(1.)).h(px(6.)).bg(rgb(0x3f3f46))
                    )
                    .child(
                        // Tick label
                        div()
                            .text_xs()
                            .text_color(rgb(0x8e8e93))
                            .mt_1()
                            .child(format!("{}s", tick_time as i32))
                    )
            );
        }

        div()
            .h(px(220.))
            .w_full()
            .bg(rgb(0x161618))
            .border_t(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col()
            .child(
                // 1. Timeline Ruler / Playhead Track
                div()
                    .h(px(40.))
                    .w_full()
                    .border_b(px(1.))
                    .border_color(rgb(0x242427))
                    .px_4()
                    .py_2()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(
                        // The relative track wrapper
                        div()
                            .relative()
                            .w_full()
                            .h(px(16.))
                            .child(
                                // Ticks markings
                                div().absolute().size_full().children(ticks)
                            )
                            .child(
                                // The interactive progress line background
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .left_0()
                                    .w_full()
                                    .h(px(4.))
                                    .bg(rgb(0x242427))
                                    .rounded_full()
                            )
                            .child(
                                // Violet colored progress bar up to current_time
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .left_0()
                                    .h(px(4.))
                                    .w(relative(progress_ratio as f32))
                                    .bg(rgb(0x8b5cf6))
                                    .rounded_full()
                            )
                            .child(
                                // Red playhead pin
                                div()
                                    .absolute()
                                    .bottom(px(-6.)) // Align vertically over the track
                                    .left(relative(progress_ratio as f32))
                                    .size(px(14.))
                                    .bg(rgb(0xef4444))
                                    .rounded_full()
                                    .border(px(2.))
                                    .border_color(rgb(0xffffff))
                            )
                    )
            )
            .child(
                // 2. Timeline Tracks Scroll Area
                div()
                    .id("timeline-tracks-scroll")
                    .overflow_y_scroll()
                    .flex_1()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .relative()
                    .children(tracks.iter().enumerate().map(|(t_idx, track)| {
                        let volume_val = track.volume.unwrap_or(1.0);
                        let volume_text = if volume_val > 0.75 {
                            "🔊 100%"
                        } else if volume_val > 0.45 {
                            "🔉 70%"
                        } else if volume_val > 0.15 {
                            "🔈 30%"
                        } else {
                            "🔇 0%"
                        };

                        div()
                            .h(px(48.))
                            .w_full()
                            .bg(rgb(0x1a1a1c))
                            .rounded(px(6.))
                            .border(px(1.))
                            .border_color(rgb(0x242427))
                            .relative()
                            .flex()
                            .items_center()
                            .child(
                                // Track Header/Indicator
                                div()
                                    .absolute()
                                    .left_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(84.))
                                    .bg(rgb(0x242427))
                                    .rounded_l(px(6.))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .border_r(px(1.))
                                    .border_color(rgb(0x1a1a1c))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x8e8e93))
                                            .child(format!("軌道 {}", t_idx + 1))
                                    )
                                    .child(
                                        // Volume Cycle Toggle Button
                                        div()
                                            .hover(|style| style.bg(rgb(0x2d2d30)))
                                            .id(("vol-btn", t_idx))
                                            .px(px(6.))
                                            .py(px(2.))
                                            .bg(rgb(0x161618))
                                            .rounded(px(3.))
                                            .cursor_pointer()
                                            .on_click(cx.listener({
                                                let core = core.clone();
                                                move |_workspace, _, _window, cx| {
                                                    let _ = core.update(cx, |core, cx| {
                                                        if let Some(track) = core.timeline.tracks.get_mut(t_idx) {
                                                            let current_vol = track.volume.unwrap_or(1.0);
                                                            let next_vol = if current_vol > 0.75 {
                                                                0.7
                                                            } else if current_vol > 0.45 {
                                                                0.3
                                                            } else if current_vol > 0.15 {
                                                                0.0
                                                            } else {
                                                                1.0
                                                            };
                                                            track.volume = Some(next_vol);
                                                            cx.notify();
                                                        }
                                                    });
                                                    cx.notify();
                                                }
                                            }))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0xc5c5c7))
                                                    .child(volume_text)
                                            )
                                    )
                            )
                            .child(
                                // Track Clips Container
                                div()
                                    .absolute()
                                    .left(px(92.)) // Offset for track header
                                    .right(px(16.))
                                    .top_0()
                                    .bottom_0()
                                    .children(track.clips.iter().enumerate().map(|(c_idx, clip)| {
                                        let start_ratio = (clip.start / duration).clamp(0.0, 1.0);
                                        let width_ratio = (clip.duration / duration).clamp(0.0, 1.0);
                                        let accent_color = get_color_by_hex(&clip.color);

                                        let clip_id = clip.id.clone();
                                        let clip_start = clip.start;
                                        let clip_duration = clip.duration;
                                        let clip_name = clip.name.clone();

                                        let selected_clip_id = self.selected_clip_id.clone();
                                        let is_selected = selected_clip_id.map(|id| id == clip.id).unwrap_or(false);

                                        let border_color = if is_selected {
                                            rgb(0xeab308) // Yellow border
                                        } else {
                                            rgb(0x2f2f33)
                                        };
                                        let border_width = if is_selected { 2. } else { 1. };

                                        let mut display_name = clip_name.clone();
                                        if let Some(ref filter) = clip.filter {
                                            let filter_zh = match filter.as_str() {
                                                "grayscale" => "黑白",
                                                "brighten" => "明亮",
                                                "contrast" => "高對比",
                                                _ => filter,
                                            };
                                            display_name.push_str(&format!(" [{}]", filter_zh));
                                        }
                                        if let Some(ref trans) = clip.transition {
                                            let trans_zh = match trans.as_str() {
                                                "fade" => "淡入淡出",
                                                _ => trans,
                                            };
                                            display_name.push_str(&format!(" ({})", trans_zh));
                                        }

                                        let waveform_overlay = if let Some(samples) = audio_waveforms.get(&clip.path) {
                                            if !samples.is_empty() {
                                                let m = 40; // 40 bars
                                                let n = samples.len();
                                                let mut bars = Vec::new();
                                                for i in 0..m {
                                                    let idx = (i * n / m).min(n - 1);
                                                    let amp = samples[idx];
                                                    let h_percent = 15.0 + amp * 60.0;
                                                    bars.push(
                                                        div()
                                                            .w(px(2.))
                                                            .h(relative(h_percent as f32 / 100.0))
                                                            .bg(rgba(0xffffff26))
                                                            .rounded_full()
                                                    );
                                                }
                                                Some(
                                                    div()
                                                        .absolute()
                                                        .size_full()
                                                        .left_0()
                                                        .top_0()
                                                        .flex()
                                                        .items_center()
                                                        .justify_around()
                                                        .px_2()
                                                        .children(bars)
                                                )
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        };

                                        div()
                                            .absolute()
                                            .left(relative(start_ratio as f32))
                                            .w(relative(width_ratio as f32))
                                            .top(px(6.))
                                            .bottom(px(6.))
                                            .bg(accent_color)
                                            .rounded(px(4.))
                                            .border(px(border_width))
                                            .border_color(border_color)
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .overflow_hidden()
                                            .child(
                                                // Left Trim Handle (6px wide)
                                                div()
                                                    .w(px(6.))
                                                    .h_full()
                                                    .bg(rgba(0xffffff26))
                                                    .hover(|s| s.bg(rgba(0xffffff66)))
                                                    .cursor_ew_resize()
                                                    .on_mouse_down(MouseButton::Left, cx.listener({
                                                        let clip_id = clip_id.clone();
                                                        move |workspace, event: &MouseDownEvent, _window, cx| {
                                                            workspace.drag_state = Some(crate::ui::workspace::DragState {
                                                                mode: crate::ui::workspace::DragMode::TrimStart {
                                                                    clip_id: clip_id.clone(),
                                                                    initial_start: clip_start,
                                                                    initial_duration: clip_duration,
                                                                },
                                                                start_mouse_x: event.position.x.into(),
                                                            });
                                                            cx.notify();
                                                        }
                                                    }))
                                            )
                                            .child(
                                                // Center Body (Moves clip)
                                                div()
                                                    .id(("clip-body", t_idx * 1000 + c_idx))
                                                    .flex_1()
                                                    .h_full()
                                                    .relative()
                                                    .flex()
                                                    .items_center()
                                                    .px_2()
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener({
                                                        let clip_id = clip_id.clone();
                                                        let is_text = clip.clip_type.as_deref() == Some("text");
                                                        move |workspace, event: &MouseDownEvent, _window, cx| {
                                                            workspace.selected_clip_id = Some(clip_id.clone());
                                                            workspace.drag_state = Some(crate::ui::workspace::DragState {
                                                                mode: crate::ui::workspace::DragMode::Move {
                                                                    clip_id: clip_id.clone(),
                                                                    initial_start: clip_start,
                                                                },
                                                                start_mouse_x: event.position.x.into(),
                                                            });
                                                            if event.click_count == 2 {
                                                                cx.spawn(move |workspace_handle: gpui::WeakEntity<crate::ui::Workspace>, cx: &mut gpui::AsyncApp| {
                                                                    let mut cx = cx.clone();
                                                                    async move {
                                                                        let _ = workspace_handle.update(&mut cx, |workspace: &mut crate::ui::Workspace, cx: &mut gpui::Context<crate::ui::Workspace>| {
                                                                            if is_text {
                                                                                workspace.active_right_tab = crate::ui::workspace::RightTab::Text;
                                                                            } else if workspace.active_right_tab == crate::ui::workspace::RightTab::Text {
                                                                                workspace.active_right_tab = crate::ui::workspace::RightTab::Transform;
                                                                            }
                                                                            cx.notify();
                                                                        });
                                                                    }
                                                                }).detach();
                                                            }
                                                            cx.notify();
                                                        }
                                                    }))
                                                    .children(waveform_overlay)
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(rgb(0xffffff))
                                                            .child(display_name)
                                                    )
                                            )
                                            .child(
                                                // Right Trim Handle (6px wide)
                                                div()
                                                    .w(px(6.))
                                                    .h_full()
                                                    .bg(rgba(0xffffff26))
                                                    .hover(|s| s.bg(rgba(0xffffff66)))
                                                    .cursor_ew_resize()
                                                    .on_mouse_down(MouseButton::Left, cx.listener({
                                                        let clip_id = clip_id.clone();
                                                        move |workspace, event: &MouseDownEvent, _window, cx| {
                                                            workspace.drag_state = Some(crate::ui::workspace::DragState {
                                                                mode: crate::ui::workspace::DragMode::TrimEnd {
                                                                    clip_id: clip_id.clone(),
                                                                    initial_duration: clip_duration,
                                                                },
                                                                start_mouse_x: event.position.x.into(),
                                                            });
                                                            cx.notify();
                                                        }
                                                    }))
                                            )
                                    }))
                            )
                    }))
                    .child(
                        // Overlay snapping indicator line (absolute positioned)
                        div()
                            .absolute()
                            .top(px(16.))
                            .bottom(px(16.))
                            .left(px(108.)) // padding 16px + track header 92px
                            .right(px(32.)) // padding 16px + track right offset 16px
                            .children(snapped_time.map(|t| {
                                let ratio = (t / duration).clamp(0.0, 1.0);
                                div()
                                    .absolute()
                                    .left(relative(ratio as f32))
                                    .top_0()
                                    .bottom_0()
                                    .w(px(2.))
                                    .bg(rgb(0xeab308)) // Yellow snapping indicator
                            }))
                    )
            )
    }
}

fn get_color_by_hex(hex: &str) -> Rgba {
    match hex {
        "#6366f1" => rgb(0x6366f1), // Indigo
        "#8b5cf6" => rgb(0x8b5cf6), // Violet
        "#10b981" => rgb(0x10b981), // Emerald
        "#f43f5e" => rgb(0xf43f5e), // Rose
        _ => rgb(0x8b5cf6),
    }
}
