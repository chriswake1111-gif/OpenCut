use crate::editor::EditorCore;
use crate::ui::media_library::MediaLibrary;
use crate::ui::player::Player;
use crate::ui::properties_panel::PropertiesPanel;
use crate::ui::timeline::Timeline;
use crate::ui::titlebar::Titlebar;
use gpui::{InteractiveElement, *};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MainMenu {
    File,
    Edit,
    View,
    Help,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportState {
    Idle,
    Exporting { progress: f32 },
    Success { file_path: PathBuf },
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragMode {
    TrimStart {
        clip_id: String,
        initial_start: f64,
        initial_duration: f64,
    },
    TrimEnd {
        clip_id: String,
        initial_duration: f64,
    },
    Move {
        clip_id: String,
        initial_start: f64,
    },
}

#[derive(Clone, Debug)]
pub struct DragState {
    pub mode: DragMode,
    pub start_mouse_x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    Assets,
    Audio,
    Text,
    Stickers,
    Effects,
    Transitions,
    Captions,
    Filters,
}

impl LeftTab {
    pub fn index(&self) -> usize {
        match self {
            LeftTab::Assets => 0,
            LeftTab::Audio => 1,
            LeftTab::Text => 2,
            LeftTab::Stickers => 3,
            LeftTab::Effects => 4,
            LeftTab::Transitions => 5,
            LeftTab::Captions => 6,
            LeftTab::Filters => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightTab {
    Transform,
    Audio,
    Speed,
    Blend,
    Mask,
    Filters,
    Text,
}

impl RightTab {
    pub fn index(&self) -> usize {
        match self {
            RightTab::Transform => 0,
            RightTab::Audio => 1,
            RightTab::Speed => 2,
            RightTab::Blend => 3,
            RightTab::Mask => 4,
            RightTab::Filters => 5,
            RightTab::Text => 6,
        }
    }
}

fn hash_str(s: &str) -> usize {
    let mut hash = 5381usize;
    for c in s.bytes() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(c as usize);
    }
    hash
}

pub struct Workspace {
    pub core: Entity<EditorCore>,
    focus_handle: FocusHandle,
    pub drag_state: Option<DragState>,
    pub active_menu: Option<MainMenu>,
    pub export_state: ExportState,
    pub selected_clip_id: Option<String>,
    pub snapped_time: Option<f64>,
    pub active_left_tab: LeftTab,
    pub active_right_tab: RightTab,
    pub snapping_enabled: bool,
    pub ripple_enabled: bool,
    pub zoom_level: f32,
    pub transcribing_state: Option<String>,
    pub ffmpeg_missing: bool,
}

impl Workspace {
    pub fn new(core: Entity<EditorCore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&core, |_, _, cx: &mut Context<Self>| {
            cx.notify();
        })
        .detach();

        let ffmpeg_missing = !crate::editor::system_check::check_ffmpeg_available()
            || !crate::editor::system_check::check_ffprobe_available();

        let focus_handle = cx.focus_handle();

        Self {
            core,
            focus_handle,
            drag_state: None,
            active_menu: None,
            export_state: ExportState::Idle,
            selected_clip_id: None,
            snapped_time: None,
            active_left_tab: LeftTab::Assets,
            active_right_tab: RightTab::Transform,
            snapping_enabled: true,
            ripple_enabled: false,
            zoom_level: 1.0,
            transcribing_state: None,
            ffmpeg_missing,
        }
    }

    fn apply_effect(
        &mut self,
        filter: Option<String>,
        transition: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref selected_id) = self.selected_clip_id {
            let core = self.core.clone();
            let selected_id = selected_id.clone();
            let _ = core.update(cx, |core, cx| {
                // Find old values
                let mut old_filter = None;
                let mut old_transition = None;
                for track in &core.timeline.tracks {
                    if let Some(clip) = track.clips.iter().find(|c| c.id == selected_id) {
                        old_filter = clip.filter.clone();
                        old_transition = clip.transition.clone();
                        break;
                    }
                }

                let new_filter = filter
                    .map(|f| if f == "none" { None } else { Some(f) })
                    .unwrap_or(old_filter.clone());
                let new_transition = transition
                    .map(|t| if t == "none" { None } else { Some(t) })
                    .unwrap_or(old_transition.clone());

                let cmd = crate::editor::ApplyEffectCommand::new(
                    selected_id,
                    old_filter,
                    old_transition,
                    new_filter,
                    new_transition,
                );
                core.execute_command(Box::new(cmd), cx);
            });
            self.active_menu = None;
            cx.notify();
        }
    }

    fn render_left_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(48.))
            .h_full()
            .bg(rgb(0x111112))
            .border_r(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col()
            .items_center()
            .py_4()
            .gap_4()
            .child(self.render_left_tab_btn(LeftTab::Assets, "📁", cx))
            .child(self.render_left_tab_btn(LeftTab::Audio, "🎵", cx))
            .child(self.render_left_tab_btn(LeftTab::Text, "💬", cx))
            .child(self.render_left_tab_btn(LeftTab::Stickers, "😊", cx))
            .child(self.render_left_tab_btn(LeftTab::Effects, "✨", cx))
            .child(self.render_left_tab_btn(LeftTab::Transitions, "🔀", cx))
            .child(self.render_left_tab_btn(LeftTab::Captions, "📝", cx))
            .child(self.render_left_tab_btn(LeftTab::Filters, "🎛", cx))
    }

    fn render_left_tab_btn(
        &self,
        tab: LeftTab,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_left_tab == tab;
        let bg_color = if is_active {
            rgb(0x242427)
        } else {
            rgb(0x111112)
        };
        let border_color = if is_active {
            rgb(0x8b5cf6)
        } else {
            rgb(0x111112)
        };

        div()
            .id(("left-tab-btn", tab.index()))
            .w_full()
            .h(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .bg(bg_color)
            .border_l(px(3.))
            .border_color(border_color)
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x1e1e20)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_left_tab = tab;
                cx.notify();
            }))
            .child(
                div()
                    .text_lg()
                    .text_color(if is_active {
                        rgb(0xffffff)
                    } else {
                        rgb(0x8e8e93)
                    })
                    .child(label),
            )
    }

    fn render_left_panel_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let panel_header = |title: &'static str| {
            div()
                .p_4()
                .border_b(px(1.))
                .border_color(rgb(0x242427))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child(title),
                )
        };

        let container = div()
            .w(px(252.))
            .h_full()
            .bg(rgb(0x161618))
            .border_r(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col();

        match self.active_left_tab {
            LeftTab::Assets => container.child(MediaLibrary::new(self.core.clone()).render(cx)),
            LeftTab::Audio => container
                .child(panel_header("音訊庫 (Audio Library)"))
                .child(
                    div()
                        .id("audio-scroll")
                        .flex_1()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .overflow_y_scroll()
                        .child(self.render_sfx_card("叮咚提示音", "2.0 秒", "dingdong.mp3", cx))
                        .child(self.render_sfx_card("相機快門聲", "1.5 秒", "shutter.mp3", cx))
                        .child(self.render_sfx_card("雨聲特效音", "10.0 秒", "rain.mp3", cx))
                        .child(self.render_sfx_card(
                            "日常Vlog輕音樂",
                            "30.0 秒",
                            "vlog_bgm.mp3",
                            cx,
                        )),
                ),
            LeftTab::Text => container
                .child(panel_header("文字與字幕 (Text & Subtitles)"))
                .child(
                    div()
                        .id("text-scroll")
                        .flex_1()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .overflow_y_scroll()
                        .child(self.render_text_tpl_card(
                            "📄 預設字幕",
                            "一般字幕樣式",
                            "預設字幕內容",
                            28.0,
                            "#ec4899",
                            cx,
                        ))
                        .child(self.render_text_tpl_card(
                            "🎬 片頭大標題",
                            "大字體粗體樣式",
                            "片頭大標題",
                            42.0,
                            "#ec4899",
                            cx,
                        ))
                        .child(self.render_text_tpl_card(
                            "🏷 精緻浮水印",
                            "半透明版權標記",
                            "Copyright @ OpenCut",
                            18.0,
                            "#ec4899",
                            cx,
                        )),
                ),
            LeftTab::Stickers => container
                .child(panel_header("貼圖 (Stickers - Mock)"))
                .child(
                    div()
                        .flex_1()
                        .p_4()
                        .text_xs()
                        .text_color(rgb(0x8e8e93))
                        .child("點擊匯入貼圖庫（目前無貼圖）"),
                ),
            LeftTab::Effects => container
                .child(panel_header("動態貼圖特效 (Effects)"))
                .child(
                    div()
                        .id("effects-scroll")
                        .flex_1()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .overflow_y_scroll()
                        .child(self.render_effect_tpl_card(
                            "落葉飄落特效",
                            "10 秒",
                            "test_leaves.mp4",
                            cx,
                        ))
                        .child(self.render_effect_tpl_card(
                            "閃亮星星特效",
                            "10 秒",
                            "test_stars.mp4",
                            cx,
                        )),
                ),
            LeftTab::Transitions => container
                .child(panel_header("轉場特效 (Transitions)"))
                .child(
                    div()
                        .id("transitions-scroll")
                        .flex_1()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .overflow_y_scroll()
                        .child(self.render_action_card(
                            "無轉場",
                            "清除轉場設定",
                            Some("none".to_string()),
                            None,
                            cx,
                        ))
                        .child(self.render_action_card(
                            "淡入淡出",
                            "套用淡入淡出轉場",
                            Some("fade".to_string()),
                            None,
                            cx,
                        )),
                ),
            LeftTab::Captions => {
                let transcribing = self.transcribing_state.clone();
                let selected_id = self.selected_clip_id.clone();
                let core = self.core.clone();

                // Find currently selected clip, or search for the first video/audio clip in tracks
                let mut target_clip = None;
                if let Some(ref sel_id) = selected_id {
                    for track in &core.read(cx).timeline.tracks {
                        if let Some(clip) = track.clips.iter().find(|c| c.id == *sel_id) {
                            if !clip.path.is_empty() {
                                target_clip = Some(clip.clone());
                            }
                            break;
                        }
                    }
                }

                // If no clip selected, find the first clip in the timeline that has a path
                if target_clip.is_none() {
                    for track in &core.read(cx).timeline.tracks {
                        if let Some(clip) = track.clips.iter().find(|c| !c.path.is_empty()) {
                            target_clip = Some(clip.clone());
                            break;
                        }
                    }
                }

                let content = if let Some(status_str) = transcribing {
                    // Loading State
                    div()
                        .flex_1()
                        .p_4()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xa78bfa))
                                .child(status_str.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8e8e93))
                                .child("本地 Whisper 辨識中，請稍候..."),
                        )
                } else if let Some(clip) = target_clip {
                    // Action State
                    let clip_name = clip.name.clone();
                    let clip_path = clip.path.clone();
                    let clip_start = clip.start;
                    let core_clone = core.clone();
                    let workspace_handle = cx.entity().clone();

                    div()
                        .flex_1()
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8e8e93))
                                .child("使用本地端 OpenAI Whisper-Base ASR 模型自動辨識音軌並產生字幕片段。")
                        )
                        .child(
                            div()
                                .p_3()
                                .bg(rgb(0x242427))
                                .rounded(px(6.))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x8e8e93))
                                        .child("辨識對象：")
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child(clip_name)
                                )
                        )
                        .child(
                            div()
                                .id("start-transcribe-btn")
                                .px_3()
                                .py_2()
                                .bg(rgb(0x8b5cf6))
                                .hover(|s| s.bg(rgb(0x7c3aed)))
                                .cursor_pointer()
                                .rounded(px(4.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click({
                                    let workspace_handle = workspace_handle.clone();
                                    let clip_path = clip_path.clone();
                                    let core = core_clone.clone();
                                    move |_, _, cx| {
                                        let workspace_handle2 = workspace_handle.clone();
                                        let _ = cx.update_entity(&workspace_handle, |this, cx| {
                                            this.transcribing_state = Some("語音分析與辨識中...".to_string());
                                            cx.notify();
                                        });

                                        let clip_path = clip_path.clone();
                                        let core = core.clone();

                                        cx.spawn(move |cx: &mut gpui::AsyncApp| {
                                            let mut cx = cx.clone();
                                            async move {
                                                let temp_json = std::env::temp_dir().join(format!(
                                                    "transcribe-{}.json",
                                                    std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_millis()
                                                ));
                                                let temp_json_str = temp_json.to_string_lossy().to_string();
                                                let input_path = clip_path.clone();
                                                let output_path = temp_json_str.clone();

                                                let success = cx.background_executor().spawn(async move {
                                                    let mut cmd = std::process::Command::new("python");
                                                    cmd.arg("tools/transcribe.py")
                                                       .arg("--input")
                                                       .arg(&input_path)
                                                       .arg("--output")
                                                       .arg(&output_path);

                                                    #[cfg(target_os = "windows")]
                                                    {
                                                        use std::os::windows::process::CommandExt;
                                                        cmd.creation_flags(0x08000000);
                                                    }

                                                    if let Ok(status) = cmd.status() {
                                                        status.success()
                                                    } else {
                                                        false
                                                    }
                                                }).await;

                                                if success {
                                                    if let Ok(content) = std::fs::read_to_string(&temp_json) {
                                                        #[derive(serde::Deserialize)]
                                                        struct TranscribeSegment {
                                                            start: f64,
                                                            end: f64,
                                                            text: String,
                                                        }

                                                        if let Ok(segments) = serde_json::from_str::<Vec<TranscribeSegment>>(&content) {
                                                            let _ = core.update(&mut cx, |core, cx| {
                                                                for segment in segments {
                                                                    let duration = segment.end - segment.start;
                                                                    if duration > 0.05 {
                                                                        let cmd = crate::editor::AddTextClipCommand::new(
                                                                            "自動字幕".to_string(),
                                                                            segment.text,
                                                                            clip_start + segment.start,
                                                                            duration,
                                                                            24.0,
                                                                            "#ffffff".to_string(),
                                                                            0,
                                                                        );
                                                                        core.execute_command(Box::new(cmd), cx);
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    }
                                                    let _ = std::fs::remove_file(&temp_json);
                                                }

                                                let _ = workspace_handle2.update(&mut cx, |workspace, cx| {
                                                    workspace.transcribing_state = None;
                                                    cx.notify();
                                                });
                                            }
                                        }).detach();
                                    }
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("⚡ 開始語音辨識")
                                )
                        )
                } else {
                    // Warning State
                    div()
                        .flex_1()
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xf43f5e))
                                .child("未發現任何可辨識的媒體。"),
                        )
                        .child(
                            div().text_xs().text_color(rgb(0x8e8e93)).child(
                                "請在時間軸上點選要辨識的影片或音訊剪輯，或先匯入素材檔案。",
                            ),
                        )
                };

                container
                    .child(panel_header("自動語音辨識字幕 (Local ASR)"))
                    .child(content)
            }
            LeftTab::Filters => container.child(panel_header("濾鏡 (Filters)")).child(
                div()
                    .id("filters-scroll")
                    .flex_1()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .overflow_y_scroll()
                    .child(self.render_action_card(
                        "無濾鏡",
                        "清除濾鏡設定",
                        None,
                        Some("none".to_string()),
                        cx,
                    ))
                    .child(self.render_action_card(
                        "黑白濾鏡",
                        "套用黑白效果",
                        None,
                        Some("grayscale".to_string()),
                        cx,
                    ))
                    .child(self.render_action_card(
                        "明亮濾鏡",
                        "提高亮度",
                        None,
                        Some("brighten".to_string()),
                        cx,
                    ))
                    .child(self.render_action_card(
                        "高對比濾鏡",
                        "增加對比度",
                        None,
                        Some("contrast".to_string()),
                        cx,
                    )),
            ),
        }
    }

    fn render_sfx_card(
        &self,
        name: &'static str,
        dur: &'static str,
        filename: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let core = self.core.clone();
        div()
            .p_3()
            .bg(rgb(0x242427))
            .rounded(px(6.))
            .border(px(1.))
            .border_color(rgb(0x2f2f33))
            .hover(|style| style.border_color(rgb(0x48484f)))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child(name),
                    )
                    .child(div().text_xs().text_color(rgb(0x8e8e93)).child(dur)),
            )
            .child(
                div()
                    .id(("add-sfx", hash_str(name)))
                    .px_2()
                    .py_0p5()
                    .bg(rgb(0x10b981))
                    .hover(|s| s.bg(rgb(0x059669)))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener({
                        let core = core.clone();
                        move |_, _, _, cx| {
                            let current_time = core.read(cx).playback.current_time;
                            let is_bgm = name.contains("音樂");
                            let track_idx = if is_bgm { 4 } else { 3 };
                            let color = if is_bgm {
                                "#3b82f6".to_string()
                            } else {
                                "#10b981".to_string()
                            };
                            let duration_sec = if name.contains("叮咚") {
                                2.0
                            } else if name.contains("快門") {
                                1.5
                            } else if name.contains("雨聲") {
                                10.0
                            } else {
                                30.0
                            };
                            let _ = core.update(cx, |core, cx| {
                                core.execute_command(
                                    Box::new(crate::editor::AddClipCommand::new(
                                        name.to_string(),
                                        format!("assets/audio/{}", filename),
                                        current_time,
                                        duration_sec,
                                        color,
                                        track_idx,
                                    )),
                                    cx,
                                );
                            });
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("+ 新增"),
                    ),
            )
    }

    fn render_text_tpl_card(
        &self,
        name: &'static str,
        desc: &'static str,
        content: &'static str,
        font_size: f64,
        color: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let core = self.core.clone();
        div()
            .p_3()
            .bg(rgb(0x242427))
            .rounded(px(6.))
            .border(px(1.))
            .border_color(rgb(0x2f2f33))
            .hover(|style| style.border_color(rgb(0x48484f)))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child(name),
                    )
                    .child(div().text_xs().text_color(rgb(0x8e8e93)).child(desc)),
            )
            .child(
                div()
                    .id(("add-txt-tpl", hash_str(name)))
                    .px_2()
                    .py_0p5()
                    .bg(rgb(0xec4899))
                    .hover(|s| s.bg(rgb(0xdb2777)))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener({
                        let core = core.clone();
                        move |_, _, _, cx| {
                            let current_time = core.read(cx).playback.current_time;
                            let _ = core.update(cx, |core, cx| {
                                core.execute_command(
                                    Box::new(crate::editor::AddTextClipCommand::new(
                                        name.replace("📄 ", "")
                                            .replace("🎬 ", "")
                                            .replace("🏷 ", "")
                                            .to_string(),
                                        content.to_string(),
                                        current_time,
                                        5.0,
                                        font_size,
                                        color.to_string(),
                                        0,
                                    )),
                                    cx,
                                );
                            });
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("+ 新增"),
                    ),
            )
    }

    fn render_effect_tpl_card(
        &self,
        name: &'static str,
        desc: &'static str,
        filename: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let core = self.core.clone();
        div()
            .p_3()
            .bg(rgb(0x242427))
            .rounded(px(6.))
            .border(px(1.))
            .border_color(rgb(0x2f2f33))
            .hover(|style| style.border_color(rgb(0x48484f)))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child(name),
                    )
                    .child(div().text_xs().text_color(rgb(0x8e8e93)).child(desc)),
            )
            .child(
                div()
                    .id(("add-effect", hash_str(name)))
                    .px_2()
                    .py_0p5()
                    .bg(rgb(0x8b5cf6))
                    .hover(|s| s.bg(rgb(0x7c3aed)))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener({
                        let core = core.clone();
                        move |_, _, _, cx| {
                            let current_time = core.read(cx).playback.current_time;
                            let _ = core.update(cx, |core, cx| {
                                core.execute_command(
                                    Box::new(crate::editor::AddClipCommand::new(
                                        name.to_string(),
                                        filename.to_string(),
                                        current_time,
                                        10.0,
                                        "#a855f7".to_string(),
                                        1,
                                    )),
                                    cx,
                                );
                            });
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("+ 新增"),
                    ),
            )
    }

    fn render_action_card(
        &self,
        name: &'static str,
        desc: &'static str,
        transition: Option<String>,
        filter: Option<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .p_3()
            .bg(rgb(0x242427))
            .rounded(px(6.))
            .border(px(1.))
            .border_color(rgb(0x2f2f33))
            .hover(|style| style.border_color(rgb(0x48484f)))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child(name),
                    )
                    .child(div().text_xs().text_color(rgb(0x8e8e93)).child(desc)),
            )
            .child(
                div()
                    .id(("apply-action", hash_str(name)))
                    .px_2()
                    .py_0p5()
                    .bg(rgb(0x8b5cf6))
                    .hover(|s| s.bg(rgb(0x7c3aed)))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener({
                        let transition = transition.clone();
                        let filter = filter.clone();
                        move |this, _, _, cx| {
                            this.apply_effect(filter.clone(), transition.clone(), cx);
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("+ 套用"),
                    ),
            )
    }

    fn render_right_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_selection = self.selected_clip_id.is_some();

        let mut is_text_clip = false;
        if let Some(ref sel_id) = self.selected_clip_id {
            let core_ref = self.core.read(cx);
            for track in &core_ref.timeline.tracks {
                if let Some(clip) = track.clips.iter().find(|c| c.id == *sel_id) {
                    if clip.clip_type.as_deref() == Some("text") {
                        is_text_clip = true;
                    }
                }
            }
        }

        let sidebar = div()
            .w(px(48.))
            .h_full()
            .bg(rgb(0x111112))
            .border_r(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col()
            .items_center()
            .py_4()
            .gap_4();

        if has_selection {
            let mut bar = sidebar
                .child(self.render_right_tab_btn(RightTab::Transform, "📐", cx))
                .child(self.render_right_tab_btn(RightTab::Audio, "🔊", cx))
                .child(self.render_right_tab_btn(RightTab::Speed, "⚡", cx))
                .child(self.render_right_tab_btn(RightTab::Blend, "💧", cx))
                .child(self.render_right_tab_btn(RightTab::Mask, "✂", cx))
                .child(self.render_right_tab_btn(RightTab::Filters, "🎨", cx));
            if is_text_clip {
                bar = bar.child(self.render_right_tab_btn(RightTab::Text, "💬", cx));
            }
            bar
        } else {
            sidebar
        }
    }

    fn render_right_tab_btn(
        &self,
        tab: RightTab,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_right_tab == tab;
        let bg_color = if is_active {
            rgb(0x242427)
        } else {
            rgb(0x111112)
        };
        let border_color = if is_active {
            rgb(0x8b5cf6)
        } else {
            rgb(0x111112)
        };

        div()
            .id(("right-tab-btn", tab.index()))
            .w_full()
            .h(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .bg(bg_color)
            .border_l(px(3.))
            .border_color(border_color)
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x1e1e20)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_right_tab = tab;
                cx.notify();
            }))
            .child(
                div()
                    .text_lg()
                    .text_color(if is_active {
                        rgb(0xffffff)
                    } else {
                        rgb(0x8e8e93)
                    })
                    .child(label),
            )
    }

    fn render_timeline_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_selection = self.selected_clip_id.is_some();
        let core = self.core.clone();

        let btn_style = |id: &'static str, label: &'static str, enabled: bool| {
            let color = if enabled {
                rgb(0xffffff)
            } else {
                rgb(0x4e4e52)
            };
            div()
                .id(id)
                .px_2()
                .py_1()
                .rounded(px(4.))
                .bg(if enabled {
                    rgb(0x242427)
                } else {
                    rgb(0x161618)
                })
                .hover(move |s| if enabled { s.bg(rgb(0x2f2f33)) } else { s })
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .child(div().text_xs().text_color(color).child(label))
        };

        let split_btn = btn_style("tb-split", "✂ 分割", true).on_click(cx.listener({
            let core = core.clone();
            move |_, _, _, cx| {
                let _ = core.update(cx, |core, cx| core.split_clip_at_playhead(cx));
            }
        }));

        let split_left_btn =
            btn_style("tb-split-left", "⇠ 剪左", has_selection).on_click(cx.listener({
                let core = core.clone();
                move |this, _, _, cx| {
                    if let Some(ref sel_id) = this.selected_clip_id {
                        let playhead = core.read(cx).playback.current_time;
                        let mut found = None;
                        for track in &core.read(cx).timeline.tracks {
                            if let Some(clip) = track.clips.iter().find(|c| c.id == *sel_id) {
                                if playhead > clip.start && playhead < clip.start + clip.duration {
                                    found = Some((clip.start, clip.duration));
                                }
                                break;
                            }
                        }
                        if let Some((_start, _duration)) = found {
                            let _ = core.update(cx, |core, cx| {
                                core.split_clip_at_playhead(cx);
                                let cmd = crate::editor::DeleteClipCommand::new(sel_id.clone());
                                core.execute_command(Box::new(cmd), cx);
                            });
                            this.selected_clip_id = None;
                            cx.notify();
                        }
                    }
                }
            }));

        let split_right_btn =
            btn_style("tb-split-right", "剪右 ⇢", has_selection).on_click(cx.listener({
                let core = core.clone();
                move |this, _, _, cx| {
                    if let Some(ref sel_id) = this.selected_clip_id {
                        let playhead = core.read(cx).playback.current_time;
                        let mut found = None;
                        for track in &core.read(cx).timeline.tracks {
                            if let Some(clip) = track.clips.iter().find(|c| c.id == *sel_id) {
                                if playhead > clip.start && playhead < clip.start + clip.duration {
                                    found = Some((clip.start, clip.duration));
                                }
                                break;
                            }
                        }
                        if let Some((_start, _duration)) = found {
                            let _ = core.update(cx, |core, cx| {
                                core.split_clip_at_playhead(cx);
                                let mut new_clip_id = None;
                                for track in &core.timeline.tracks {
                                    if let Some(clip) = track
                                        .clips
                                        .iter()
                                        .find(|c| (c.start - playhead).abs() < 0.01)
                                    {
                                        new_clip_id = Some(clip.id.clone());
                                        break;
                                    }
                                }
                                if let Some(right_id) = new_clip_id {
                                    let cmd = crate::editor::DeleteClipCommand::new(right_id);
                                    core.execute_command(Box::new(cmd), cx);
                                }
                            });
                            this.selected_clip_id = None;
                            cx.notify();
                        }
                    }
                }
            }));

        let duplicate_btn =
            btn_style("tb-duplicate", "📋 複製", has_selection).on_click(cx.listener({
                let core = core.clone();
                move |this, _, _, cx| {
                    if let Some(ref sel_id) = this.selected_clip_id {
                        let mut clip_to_dup = None;
                        let mut track_idx = 0;
                        for (t_idx, track) in core.read(cx).timeline.tracks.iter().enumerate() {
                            if let Some(clip) = track.clips.iter().find(|c| c.id == *sel_id) {
                                clip_to_dup = Some(clip.clone());
                                track_idx = t_idx;
                                break;
                            }
                        }
                        if let Some(clip) = clip_to_dup {
                            let playhead = core.read(cx).playback.current_time;
                            let _ = core.update(cx, |core, cx| {
                                if clip.clip_type.as_deref() == Some("text") {
                                    let cmd = crate::editor::AddTextClipCommand::new(
                                        format!("{} 複製", clip.name),
                                        clip.text_content.clone().unwrap_or_default(),
                                        playhead,
                                        clip.duration,
                                        clip.font_size.unwrap_or(28.0) as f64,
                                        clip.color.clone(),
                                        track_idx,
                                    );
                                    core.execute_command(Box::new(cmd), cx);
                                } else {
                                    let cmd = crate::editor::AddClipCommand::new(
                                        format!("{} 複製", clip.name),
                                        clip.path.clone(),
                                        playhead,
                                        clip.duration,
                                        clip.color.clone(),
                                        track_idx,
                                    );
                                    core.execute_command(Box::new(cmd), cx);
                                }
                            });
                        }
                    }
                }
            }));

        let delete_btn = btn_style("tb-delete", "🗑 刪除", has_selection).on_click(cx.listener({
            let core = core.clone();
            move |this, _, _, cx| {
                if let Some(ref sel_id) = this.selected_clip_id {
                    let _ = core.update(cx, |core, cx| {
                        let cmd = crate::editor::DeleteClipCommand::new(sel_id.clone());
                        core.execute_command(Box::new(cmd), cx);
                    });
                    this.selected_clip_id = None;
                    cx.notify();
                }
            }
        }));

        let snap_bg = if self.snapping_enabled {
            rgb(0x8b5cf6)
        } else {
            rgb(0x242427)
        };
        let snap_color = if self.snapping_enabled {
            rgb(0xffffff)
        } else {
            rgb(0x8e8e93)
        };
        let snap_btn = div()
            .id("tb-snap")
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(snap_bg)
            .hover(|s| s.bg(rgb(0x7c3aed)))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.snapping_enabled = !this.snapping_enabled;
                cx.notify();
            }))
            .child(div().text_xs().text_color(snap_color).child("🧲 磁吸對齊"));

        let ripple_bg = if self.ripple_enabled {
            rgb(0x8b5cf6)
        } else {
            rgb(0x242427)
        };
        let ripple_color = if self.ripple_enabled {
            rgb(0xffffff)
        } else {
            rgb(0x8e8e93)
        };
        let ripple_btn = div()
            .id("tb-ripple")
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(ripple_bg)
            .hover(|s| s.bg(rgb(0x7c3aed)))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.ripple_enabled = !this.ripple_enabled;
                cx.notify();
            }))
            .child(
                div()
                    .text_xs()
                    .text_color(ripple_color)
                    .child("🌊 漣漪編輯"),
            );

        let zoom_out = div()
            .id("tb-zoom-out")
            .w(px(28.))
            .h(px(20.))
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .bg(rgb(0x242427))
            .hover(|s| s.bg(rgb(0x2f2f33)))
            .cursor_pointer()
            .rounded(px(3.))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.zoom_level = (this.zoom_level - 0.25).max(1.0);
                cx.notify();
            }))
            .child(div().text_xs().text_color(rgb(0xffffff)).child("🔍⁻"));

        let zoom_in = div()
            .id("tb-zoom-in")
            .w(px(28.))
            .h(px(20.))
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .bg(rgb(0x242427))
            .hover(|s| s.bg(rgb(0x2f2f33)))
            .cursor_pointer()
            .rounded(px(3.))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.zoom_level = (this.zoom_level + 0.25).min(4.0);
                cx.notify();
            }))
            .child(div().text_xs().text_color(rgb(0xffffff)).child("🔍⁺"));

        let slider_progress = (self.zoom_level - 1.0) / 3.0;
        let slider_width = 80.0;
        let thumb_pos = slider_progress * slider_width;

        let zoom_slider = div()
            .w(px(slider_width as f32))
            .h(px(4.))
            .flex_shrink_0()
            .bg(rgb(0x242427))
            .relative()
            .rounded_full()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .h_full()
                    .bg(rgb(0x8b5cf6))
                    .w(relative(slider_progress)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(thumb_pos as f32 - 4.))
                    .top(px(-2.))
                    .size(px(8.))
                    .rounded_full()
                    .bg(rgb(0xffffff))
                    .border(px(1.))
                    .border_color(rgb(0x8b5cf6)),
            );

        div()
            .w_full()
            .h(px(36.))
            .bg(rgb(0x161618))
            .border_t(px(1.))
            .border_b(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(split_btn)
                    .child(split_left_btn)
                    .child(split_right_btn)
                    .child(duplicate_btn)
                    .child(delete_btn),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(snap_btn)
                    .child(ripple_btn)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .flex_shrink_0()
                            .gap_1_5()
                            .child(zoom_out)
                            .child(zoom_slider)
                            .child(zoom_in),
                    ),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus_handle);

        let workspace_handle = cx.entity().clone();
        let workspace_handle_clone = workspace_handle.clone();
        let workspace_handle_clone2 = workspace_handle.clone();

        let active_menu = self.active_menu;
        let export_state = self.export_state.clone();

        let ffmpeg_banner = if self.ffmpeg_missing {
            Some(
                div()
                    .id("ffmpeg-missing-banner")
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(0xd97706)) // Amber-600 黃橘色
                    .py_2()
                    .px_4()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .font_weight(FontWeight::BOLD)
                            .child("⚠️ 系統未檢測到 FFmpeg 或 FFprobe。影片匯出功能將無法正常運作。請安裝 FFmpeg 並將其加入 PATH。")
                    )
            )
        } else {
            None
        };

        // Dropdown menus absolute overlay
        let file_menu = if active_menu == Some(MainMenu::File) {
            Some(
                div()
                    .absolute()
                    .top(px(40.))
                    .left(px(165.))
                    .w(px(180.))
                    .bg(rgb(0x1a1a1c))
                    .border(px(1.))
                    .border_color(rgb(0x2d2d30))
                    .rounded(px(4.))
                    .p_1()
                    .flex()
                    .flex_col()
                    .child(menu_option("開新專案 (Ctrl+N)", crate::NewProject))
                    .child(menu_option("開啟專案 (Ctrl+O)", crate::OpenProject))
                    .child(menu_option("儲存專案 (Ctrl+S)", crate::SaveProject))
                    .child(menu_option("匯入媒體 (Ctrl+I)", crate::ImportClip))
                    .child(menu_option("載入範例素材 (Ctrl+D)", crate::LoadDemo))
                    .child(menu_option("匯出影片 (Ctrl+E)", crate::ExportVideo)),
            )
        } else {
            None
        };

        let edit_menu = if active_menu == Some(MainMenu::Edit) {
            Some(
                div()
                    .absolute()
                    .top(px(40.))
                    .left(px(205.))
                    .w(px(200.))
                    .bg(rgb(0x1a1a1c))
                    .border(px(1.))
                    .border_color(rgb(0x2d2d30))
                    .rounded(px(4.))
                    .p_1()
                    .flex()
                    .flex_col()
                    .child(menu_option("復原 (Ctrl+Z)", crate::Undo))
                    .child(menu_option("重做 (Ctrl+Y)", crate::Redo))
                    .child(menu_option("分割剪輯 (Ctrl+K)", crate::Split))
                    .child(menu_option("刪除選取剪輯 (Delete)", crate::DeleteClip))
                    .child(div().h(px(1.)).bg(rgb(0x242427)).my_1())
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(0x8e8e93))
                            .child("套用色彩濾鏡："),
                    )
                    .child(menu_option("  ↳ 無濾鏡", crate::ApplyNoneFilter))
                    .child(menu_option("  ↳ 黑白濾鏡", crate::ApplyGrayscaleFilter))
                    .child(menu_option("  ↳ 明亮濾鏡", crate::ApplyBrightenFilter))
                    .child(menu_option("  ↳ 高對比濾鏡", crate::ApplyContrastFilter))
                    .child(div().h(px(1.)).bg(rgb(0x242427)).my_1())
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(0x8e8e93))
                            .child("套用剪輯轉場："),
                    )
                    .child(menu_option("  ↳ 無轉場", crate::ApplyNoneTransition))
                    .child(menu_option("  ↳ 淡入淡出轉場", crate::ApplyFadeTransition)),
            )
        } else {
            None
        };

        // Export overlay modal
        let export_overlay = match export_state {
            ExportState::Idle => None,
            ExportState::Exporting { progress } => {
                Some(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgba(0x0a0a0cee))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(380.))
                                .bg(rgb(0x161618))
                                .border(px(1.))
                                .border_color(rgb(0x2f2f33))
                                .rounded(px(8.))
                                .p_6()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_4()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("影片匯出中...")
                                )
                                .child(
                                    // Progress bar track
                                    div()
                                        .h(px(6.))
                                        .w_full()
                                        .bg(rgb(0x242427))
                                        .rounded_full()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .h_full()
                                                .bg(rgb(0x8b5cf6))
                                                .rounded_full()
                                                .w(relative(progress))
                                        )
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x8e8e93))
                                        .child(format!("{:.0}%", progress * 100.0))
                                )
                        )
                )
            }
            ExportState::Success { file_path } => {
                let file_path_clone = file_path.clone();
                Some(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgba(0x0a0a0cee))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(380.))
                                .bg(rgb(0x161618))
                                .border(px(1.))
                                .border_color(rgb(0x2f2f33))
                                .rounded(px(8.))
                                .p_6()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_4()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x10b981))
                                        .child("🎉 影片匯出成功！")
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xc5c5c7))
                                        .text_center()
                                        .child(file_path.to_string_lossy().to_string())
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_4()
                                        .child(
                                            div()
                                                .id("open-folder-btn")
                                                .px_4()
                                                .py_1_5()
                                                .bg(rgb(0x242427))
                                                .hover(|s| s.bg(rgb(0x2f2f33)))
                                                .rounded(px(4.))
                                                .cursor_pointer()
                                                .on_click(move |_, _, cx| {
                                                    cx.reveal_path(&file_path_clone);
                                                })
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .child("打開資料夾")
                                                )
                                        )
                                        .child(
                                            div()
                                                .id("confirm-success-btn")
                                                .px_4()
                                                .py_1_5()
                                                .bg(rgb(0x8b5cf6))
                                                .hover(|s| s.bg(rgb(0x7c3aed)))
                                                .rounded(px(4.))
                                                .cursor_pointer()
                                                .on_click(move |_, _, cx| {
                                                    let _ = cx.update_entity(&workspace_handle_clone, |workspace: &mut Workspace, cx: &mut Context<Workspace>| {
                                                        workspace.export_state = ExportState::Idle;
                                                        cx.notify();
                                                    });
                                                })
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .child("確定")
                                                )
                                        )
                                )
                        )
                )
            }
            ExportState::Failed(err) => {
                Some(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgba(0x0a0a0cee))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(380.))
                                .bg(rgb(0x161618))
                                .border(px(1.))
                                .border_color(rgb(0x2f2f33))
                                .rounded(px(8.))
                                .p_6()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_4()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xef4444))
                                        .child("❌ 影片匯出失敗")
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xef4444))
                                        .text_center()
                                        .child(err)
                                )
                                .child(
                                    div()
                                        .id("confirm-failed-btn")
                                        .px_4()
                                        .py_1_5()
                                        .bg(rgb(0x242427))
                                        .hover(|s| s.bg(rgb(0x2f2f33)))
                                        .rounded(px(4.))
                                        .cursor_pointer()
                                        .on_click(move |_, _, cx| {
                                            let _ = cx.update_entity(&workspace_handle_clone2, |workspace: &mut Workspace, cx: &mut Context<Workspace>| {
                                                workspace.export_state = ExportState::Idle;
                                                cx.notify();
                                            });
                                        })
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .child("確定")
                                        )
                                )
                        )
                )
            }
        };

        div()
            .id("workspace-root")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &crate::Undo, _, cx| {
                let _ = this.core.update(cx, |core, cx| core.undo(cx));
            }))
            .on_action(cx.listener(|this, _: &crate::Redo, _, cx| {
                let _ = this.core.update(cx, |core, cx| core.redo(cx));
            }))
            .on_action(cx.listener(|this, _: &crate::Split, _, cx| {
                let _ = this.core.update(cx, |core, cx| core.split_clip_at_playhead(cx));
            }))
            .on_action(cx.listener(|this, _: &crate::ToggleSnapping, _, cx| {
                this.snapping_enabled = !this.snapping_enabled;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::NewProject, _, cx| {
                this.selected_clip_id = None;
                let _ = this.core.update(cx, |core, cx| core.new_project(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::LoadDemo, _, cx| {
                this.selected_clip_id = None;
                let _ = this.core.update(cx, |core, cx| core.load_demo(cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::TogglePlay, _, cx| {
                let _ = this.core.update(cx, |core, cx| {
                    if core.playback.is_playing {
                        core.pause(cx);
                    } else {
                        core.play(cx);
                    }
                });
            }))
            .on_action(cx.listener(|this, _: &crate::SeekForward, _, cx| {
                let _ = this.core.update(cx, |core, cx| {
                    let t = core.playback.current_time + 1.0;
                    core.seek(t, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &crate::SeekBackward, _, cx| {
                let _ = this.core.update(cx, |core, cx| {
                    let t = core.playback.current_time - 1.0;
                    core.seek(t, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &crate::FrameStepForward, _, cx| {
                let _ = this.core.update(cx, |core, cx| {
                    let t = core.playback.current_time + (1.0 / 30.0);
                    core.seek(t, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &crate::FrameStepBackward, _, cx| {
                let _ = this.core.update(cx, |core, cx| {
                    let t = core.playback.current_time - (1.0 / 30.0);
                    core.seek(t, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &crate::DeleteClip, _, cx| {
                if let Some(ref selected_id) = this.selected_clip_id {
                    let core = this.core.clone();
                    let selected_id = selected_id.clone();
                    let _ = core.update(cx, |core, cx| {
                        let cmd = crate::editor::DeleteClipCommand::new(selected_id);
                        core.execute_command(Box::new(cmd), cx);
                    });
                    this.selected_clip_id = None;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &crate::ImportClip, _, cx| {
                let core = this.core.clone();
                let task = cx.prompt_for_paths(gpui::PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("選擇要新增的影片或音訊檔案".into()),
                });
                cx.spawn(|_this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Ok(Ok(Some(paths))) = task.await {
                            if let Some(path) = paths.first() {
                                let file_name = path.file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("新增剪輯.mp4")
                                    .to_string();
                                let absolute_path = path.to_string_lossy().to_string();
                                let lower_name = file_name.to_lowercase();
                                let is_audio = lower_name.ends_with(".mp3") ||
                                               lower_name.ends_with(".wav") ||
                                               lower_name.ends_with(".aac") ||
                                               lower_name.ends_with(".m4a");
                                let track_index = if is_audio { 2 } else { 0 };
                                let color = if is_audio { "#10b981".to_string() } else { "#f43f5e".to_string() };

                                let _ = core.update(&mut cx, |core, cx| {
                                    core.execute_command(
                                        Box::new(crate::editor::AddClipCommand::new(
                                            file_name,
                                            absolute_path,
                                            15.0,
                                            8.0,
                                            color,
                                            track_index,
                                        )),
                                        cx,
                                    );
                                });
                            }
                        }
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyGrayscaleFilter, _, cx| {
                this.apply_effect(Some("grayscale".to_string()), None, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyBrightenFilter, _, cx| {
                this.apply_effect(Some("brighten".to_string()), None, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyContrastFilter, _, cx| {
                this.apply_effect(Some("contrast".to_string()), None, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyNoneFilter, _, cx| {
                this.apply_effect(Some("none".to_string()), None, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyFadeTransition, _, cx| {
                this.apply_effect(None, Some("fade".to_string()), cx);
            }))
            .on_action(cx.listener(|this, _: &crate::ApplyNoneTransition, _, cx| {
                this.apply_effect(None, Some("none".to_string()), cx);
            }))
            .on_action(cx.listener(|this, _: &crate::OpenProject, _, cx| {
                this.active_menu = None;
                let task = cx.prompt_for_paths(gpui::PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("選擇要開啟的專案檔案 (.opencut)".into()),
                });
                cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Ok(Ok(Some(paths))) = task.await {
                            if let Some(path) = paths.first() {
                                let _ = this.update(&mut cx, |workspace: &mut Self, cx: &mut gpui::Context<Self>| {
                                    let _ = workspace.core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                        let _ = core.load_project(path, cx);
                                    });
                                });
                            }
                        }
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this, _: &crate::SaveProject, _, cx| {
                this.active_menu = None;
                let task = cx.prompt_for_new_path(
                    &std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()),
                    Some("project.opencut"),
                );
                cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Ok(Ok(Some(path))) = task.await {
                            let _ = this.update(&mut cx, |workspace: &mut Self, cx: &mut gpui::Context<Self>| {
                                let _ = workspace.core.update(cx, |core: &mut EditorCore, _cx: &mut gpui::Context<EditorCore>| {
                                    let _ = core.save_project(&path);
                                });
                            });
                        }
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this, _: &crate::ExportVideo, _, cx| {
                this.active_menu = None;
                let task = cx.prompt_for_new_path(
                    &std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()),
                    Some("export.mp4"),
                );
                let handle = cx.entity().downgrade();
                cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Ok(Ok(Some(path))) = task.await {
                            let _ = this.update(&mut cx, |workspace: &mut Self, cx: &mut gpui::Context<Self>| {
                                workspace.export_state = ExportState::Exporting { progress: 0.0 };
                                let core = workspace.core.clone();
                                let handle_clone = handle.clone();
                                let _ = core.update(cx, |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                                    core.export_video(&path, handle_clone, cx);
                                });
                                cx.notify();
                            });
                        }
                    }
                }).detach();
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                // Clicking anywhere closes active menu dropdown
                if this.active_menu.is_some() {
                    this.active_menu = None;
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if let Some(ref drag) = this.drag_state {
                    let window_width: f32 = window.bounds().size.width.into();
                    let timeline_width = (window_width - 72.0).max(100.0);
                    let delta_x = f32::from(event.position.x) - drag.start_mouse_x;

                    let duration = this.core.read(cx).playback.duration;
                    let delta_seconds = (delta_x / timeline_width) as f64 * duration;

                    let mut snapped_time = None;

                    let _ = this.core.update(cx, |core, cx| {
                        // 1. Collect snap targets (exclude the clip being dragged)
                        let mut snap_targets = Vec::new();
                        // Add playhead
                        snap_targets.push(core.playback.current_time);

                        // Extract target clip_id from drag mode
                        let current_drag_clip_id = match &drag.mode {
                            DragMode::Move { clip_id, .. } => Some(clip_id.clone()),
                            DragMode::TrimStart { clip_id, .. } => Some(clip_id.clone()),
                            DragMode::TrimEnd { clip_id, .. } => Some(clip_id.clone()),
                        };

                        if let Some(ref target_id) = current_drag_clip_id {
                            for track in &core.timeline.tracks {
                                for clip in &track.clips {
                                    if clip.id != *target_id {
                                        snap_targets.push(clip.start);
                                        snap_targets.push(clip.start + clip.duration);
                                    }
                                }
                            }
                        }

                        let snap_threshold = 0.15; // 150ms

                        // Helper closure to find the closest snap target
                        let find_snap = |time: f64| -> Option<f64> {
                            let mut closest_target: Option<f64> = None;
                            let mut min_diff = snap_threshold;
                            for &target in &snap_targets {
                                let diff = (time - target).abs();
                                if diff < min_diff {
                                    min_diff = diff;
                                    closest_target = Some(target);
                                }
                            }
                            closest_target
                        };

                        match &drag.mode {
                            DragMode::Move { clip_id, initial_start } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_start = (initial_start + delta_seconds).max(0.0);
                                        let target_end = target_start + clip.duration;

                                        // Check snap for start or end
                                        if let Some(snap_start) = find_snap(target_start) {
                                            clip.start = snap_start;
                                            snapped_time = Some(snap_start);
                                        } else if let Some(snap_end) = find_snap(target_end) {
                                            clip.start = (snap_end - clip.duration).max(0.0);
                                            snapped_time = Some(snap_end);
                                        } else {
                                            clip.start = target_start;
                                        }
                                        core.update_frame(cx);
                                        break;
                                    }
                                }
                            }
                            DragMode::TrimStart { clip_id, initial_start, initial_duration } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_start = (initial_start + delta_seconds).clamp(0.0, initial_start + initial_duration - 0.5);

                                        let final_start = if let Some(snap_start) = find_snap(target_start) {
                                            let clamped = snap_start.clamp(0.0, initial_start + initial_duration - 0.5);
                                            snapped_time = Some(clamped);
                                            clamped
                                        } else {
                                            target_start
                                        };

                                        clip.start = final_start;
                                        clip.duration = initial_duration - (final_start - initial_start);
                                        core.update_frame(cx);
                                        break;
                                    }
                                }
                            }
                            DragMode::TrimEnd { clip_id, initial_duration } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_duration = (initial_duration + delta_seconds).max(0.5);
                                        let target_end = clip.start + target_duration;

                                        if let Some(snap_end) = find_snap(target_end) {
                                            let final_duration = (snap_end - clip.start).max(0.5);
                                            clip.duration = final_duration;
                                            snapped_time = Some(clip.start + final_duration);
                                        } else {
                                            clip.duration = target_duration;
                                        }
                                        core.update_frame(cx);
                                        break;
                                    }
                                }
                            }
                        }
                        cx.notify();
                    });

                    this.snapped_time = snapped_time;
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &MouseUpEvent, window, cx| {
                if let Some(drag) = this.drag_state.take() {
                    let window_width: f32 = window.bounds().size.width.into();
                    let timeline_width = (window_width - 72.0).max(100.0);
                    let delta_x = f32::from(event.position.x) - drag.start_mouse_x;

                    let duration = this.core.read(cx).playback.duration;
                    let delta_seconds = (delta_x / timeline_width) as f64 * duration;

                    let _ = this.core.update(cx, |core, cx| {
                        // 1. Collect snap targets (exclude target clip)
                        let mut snap_targets = Vec::new();
                        snap_targets.push(core.playback.current_time);

                        let current_drag_clip_id = match &drag.mode {
                            DragMode::Move { clip_id, .. } => Some(clip_id.clone()),
                            DragMode::TrimStart { clip_id, .. } => Some(clip_id.clone()),
                            DragMode::TrimEnd { clip_id, .. } => Some(clip_id.clone()),
                        };

                        if let Some(ref target_id) = current_drag_clip_id {
                            for track in &core.timeline.tracks {
                                for clip in &track.clips {
                                    if clip.id != *target_id {
                                        snap_targets.push(clip.start);
                                        snap_targets.push(clip.start + clip.duration);
                                    }
                                }
                            }
                        }

                        let snap_threshold = 0.15;

                        let find_snap = |time: f64| -> Option<f64> {
                            let mut closest_target: Option<f64> = None;
                            let mut min_diff = snap_threshold;
                            for &target in &snap_targets {
                                let diff = (time - target).abs();
                                if diff < min_diff {
                                    min_diff = diff;
                                    closest_target = Some(target);
                                }
                            }
                            closest_target
                        };

                        match &drag.mode {
                            DragMode::Move { clip_id, initial_start } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_start = (initial_start + delta_seconds).max(0.0);
                                        let target_end = target_start + clip.duration;

                                        let final_start = if let Some(snap_start) = find_snap(target_start) {
                                            snap_start
                                        } else if let Some(snap_end) = find_snap(target_end) {
                                            (snap_end - clip.duration).max(0.0)
                                        } else {
                                            target_start
                                        };

                                        // Reset to initial to allow execute_command to record the change
                                        clip.start = *initial_start;

                                        let cmd = crate::editor::EditClipCommand::new(
                                            clip_id.clone(),
                                            *initial_start,
                                            clip.duration,
                                            final_start,
                                            clip.duration,
                                        );
                                        core.execute_command(Box::new(cmd), cx);
                                        break;
                                    }
                                }
                            }
                            DragMode::TrimStart { clip_id, initial_start, initial_duration } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_start = (initial_start + delta_seconds).clamp(0.0, initial_start + initial_duration - 0.5);

                                        let final_start = if let Some(snap_start) = find_snap(target_start) {
                                            snap_start.clamp(0.0, initial_start + initial_duration - 0.5)
                                        } else {
                                            target_start
                                        };

                                        let final_duration = initial_duration - (final_start - initial_start);

                                        // Reset to initial values
                                        clip.start = *initial_start;
                                        clip.duration = *initial_duration;

                                        let cmd = crate::editor::EditClipCommand::new(
                                            clip_id.clone(),
                                            *initial_start,
                                            *initial_duration,
                                            final_start,
                                            final_duration,
                                        );
                                        core.execute_command(Box::new(cmd), cx);
                                        break;
                                    }
                                }
                            }
                            DragMode::TrimEnd { clip_id, initial_duration } => {
                                for track in &mut core.timeline.tracks {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                        let target_duration = (initial_duration + delta_seconds).max(0.5);
                                        let target_end = clip.start + target_duration;

                                        let final_duration = if let Some(snap_end) = find_snap(target_end) {
                                            (snap_end - clip.start).max(0.5)
                                        } else {
                                            target_duration
                                        };

                                        // Reset to initial values
                                        clip.duration = *initial_duration;

                                        let cmd = crate::editor::EditClipCommand::new(
                                            clip_id.clone(),
                                            clip.start,
                                            *initial_duration,
                                            clip.start,
                                            final_duration,
                                        );
                                        core.execute_command(Box::new(cmd), cx);
                                        break;
                                    }
                                }
                            }
                        }
                    });

                    this.snapped_time = None;
                    cx.notify();
                }
            }))
            .size_full()
            .bg(rgb(0x0b0b0c))
            .flex()
            .flex_col()
            .child(Titlebar::new(active_menu).render(cx))
            .children(ffmpeg_banner)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .child(self.render_left_sidebar(cx))
                    .child(self.render_left_panel_content(cx))
                    .child(Player::new(self.core.clone()).render(cx))
                    .child(self.render_right_sidebar(cx))
                    .child(PropertiesPanel::new(self.core.clone(), self.selected_clip_id.clone(), self.active_right_tab).render(cx))
            )
            .child(
                self.render_timeline_toolbar(cx)
            )
            .child(
                Timeline::new(self.core.clone(), self.selected_clip_id.clone(), self.snapped_time).render(cx)
            )
            .children(file_menu)
            .children(edit_menu)
            .children(export_overlay)
    }
}

fn menu_option<A: Action + Clone>(label: &'static str, action: A) -> impl IntoElement {
    div()
        .id(label)
        .px_3()
        .py_1_5()
        .rounded(px(4.))
        .text_xs()
        .text_color(rgb(0xc5c5c7))
        .hover(|style| style.bg(rgb(0x242427)).text_color(rgb(0xffffff)))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            cx.dispatch_action(&action);
        })
        .child(label)
}
