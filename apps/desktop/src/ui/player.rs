use gpui::{*, InteractiveElement};
use crate::editor::EditorCore;

pub struct Player {
    core: Entity<EditorCore>,
}

impl Player {
    pub fn new(core: Entity<EditorCore>) -> Self {
        Self { core }
    }

    pub fn render(self, cx: &mut App) -> impl IntoElement {
        let core = self.core.clone();
        let is_playing = core.read(cx).playback.is_playing;
        let current_time = core.read(cx).playback.current_time;
        let duration = core.read(cx).playback.duration;

        let current_frame = core.read(cx).current_frame.clone();

        // Fetch active text clips
        let mut active_texts = Vec::new();
        {
            let core_ref = core.read(cx);
            for track in &core_ref.timeline.tracks {
                for clip in &track.clips {
                    if clip.clip_type.as_deref() == Some("text")
                        && current_time >= clip.start
                        && current_time <= clip.start + clip.duration
                    {
                        active_texts.push(clip.clone());
                    }
                }
            }
        }

        // Generate 36 dynamic equalizer bars to simulate audio/video waveform
        let mut waveform_bars = Vec::new();
        for i in 0..36 {
            let time_factor = if is_playing { current_time * 8.0 } else { 0.0 };
            let angle = (i as f64 * 0.4) + time_factor;
            let sine_val = angle.sin().abs();
            let height_val = 12.0 + (sine_val * 48.0);

            let progress_ratio = current_time / duration;
            let bar_ratio = i as f64 / 36.0;
            let is_past_playhead = bar_ratio <= progress_ratio;

            let bar_color = if is_past_playhead {
                rgb(0x8b5cf6) // Violet for played path
            } else {
                rgb(0x48484f) // Zinc/gray for unplayed path
            };

            waveform_bars.push(
                div()
                    .w(px(6.))
                    .h(px(height_val as f32))
                    .bg(bar_color)
                    .rounded_full()
            );
        }

        div()
            .flex_1()
            .h_full()
            .bg(rgb(0x0b0b0c))
            .flex()
            .flex_col()
            .justify_between()
            .p_6()
            .child(
                // Player Screen / Preview Area
                div()
                    .flex_1()
                    .w_full()
                    .bg(rgb(0x101012))
                    .rounded(px(8.))
                    .border(px(1.))
                    .border_color(rgb(0x242427))
                    .overflow_hidden()
                    .relative()
                    .child(
                        if let Some(frame) = current_frame {
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    img(gpui::ImageSource::Render(frame))
                                        .w_full()
                                        .h_full()
                                )
                        } else {
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_6()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x8e8e93))
                                        .child("影片預覽（模擬訊號）")
                                )
                                .child(
                                    div()
                                        .h(px(80.))
                                        .flex()
                                        .items_end()
                                        .gap(px(4.))
                                        .children(waveform_bars)
                                )
                        }
                    )
                    .children(
                        active_texts.into_iter().map(|clip| {
                            let text_content = clip.text_content.unwrap_or_else(|| "文字".to_string());
                            let font_size = clip.font_size.unwrap_or(28.0);
                            let color_str = clip.text_color.unwrap_or_else(|| "#ffffff".to_string());
                            let opacity_val = clip.opacity.unwrap_or(1.0) as f32;

                            let text_color = parse_hex_color(&color_str);

                            let px_val = clip.position_x.unwrap_or(0.0) as f32;
                            let py_val = clip.position_y.unwrap_or(0.0) as f32;

                            div()
                                .absolute()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .ml(px(px_val))
                                        .mt(px(py_val))
                                        .text_color(text_color)
                                        .text_size(px(font_size))
                                        .font_weight(FontWeight::BOLD)
                                        .opacity(opacity_val)
                                        .child(text_content)
                                )
                        })
                    )
            )
            .child(
                // Player Controls & Progress bar
                div()
                    .h(px(80.))
                    .w_full()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .gap_3()
                    .child(
                        // Control buttons and Timecode
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                // Left: Timecode
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(format!(
                                        "{} / {}",
                                        format_time(current_time),
                                        format_time(duration)
                                    ))
                            )
                            .child(
                                // Center: Playback Control Buttons
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_4()
                                    // Rewind 5s
                                    .child(control_btn("⏮", {
                                        let core = core.clone();
                                        move |cx| {
                                            let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                                let t = core.playback.current_time - 5.0;
                                                core.seek(t, cx);
                                            });
                                        }
                                    }))
                                    // Play / Pause
                                    .child(
                                        div()
                                            .id("play-pause-btn")
                                            .size_10()
                                            .rounded_full()
                                            .bg(rgb(0x8b5cf6))
                                            .hover(|style| style.bg(rgb(0x7c3aed)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .on_click({
                                                let core = core.clone();
                                                move |_, _, cx| {
                                                    println!("Play/Pause button clicked! is_playing: {}", is_playing);
                                                    let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                                        if is_playing {
                                                            core.pause(cx);
                                                        } else {
                                                            core.play(cx);
                                                        }
                                                    });
                                                }
                                            })
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .text_color(rgb(0xffffff))
                                                    .child(if is_playing { "⏸" } else { "▶" })
                                            )
                                    )
                                    // Stop
                                    .child(control_btn("⏹", {
                                        let core = core.clone();
                                        move |cx| {
                                            let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                                core.pause(cx);
                                                core.seek(0.0, cx);
                                            });
                                        }
                                    }))
                                    // Fast Forward 5s
                                    .child(control_btn("⏭", {
                                        let core = core.clone();
                                        move |cx| {
                                            let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                                let t = core.playback.current_time + 5.0;
                                                core.seek(t, cx);
                                            });
                                        }
                                    }))
                            )
                            .child(
                                // Right: Status Tag
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.))
                                    .bg(if is_playing { rgb(0x059669) } else { rgb(0x3f3f46) })
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xffffff))
                                            .child(if is_playing { "播放中" } else { "已暫停" })
                                    )
                            )
                    )
            )
    }
}

fn control_btn<F>(icon: &'static str, action: F) -> impl IntoElement
where
    F: 'static + Fn(&mut App) + Send + Sync,
{
    div()
        .id(icon)
        .size_8()
        .rounded_full()
        .bg(rgb(0x242427))
        .hover(|style| style.bg(rgb(0x2f2f33)))
        .flex()
        .items_center()
        .justify_center()
        .on_click(move |_, _, cx| action(cx))
        .child(
            div()
                .text_sm()
                .text_color(rgb(0xc5c5c7))
                .child(icon)
        )
}

fn format_time(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor() as i32;
    let secs = (seconds % 60.0).floor() as i32;
    let millis = ((seconds % 1.0) * 100.0).floor() as i32;
    format!("{:02}:{:02}.{:02}", minutes, secs, millis)
}

fn parse_hex_color(hex: &str) -> gpui::Rgba {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return gpui::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32);
        }
    }
    gpui::rgb(0xffffff) // Default white
}
