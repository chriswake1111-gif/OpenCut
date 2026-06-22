use gpui::{*, InteractiveElement};
use crate::editor::{EditorCore, EditClipTransformCommand, EditClipTextCommand, EditClipAudioCommand, EditClipBlendModeCommand, Clip};

pub struct PropertiesPanel {
    core: Entity<EditorCore>,
    selected_clip_id: Option<String>,
    active_right_tab: crate::ui::workspace::RightTab,
}

impl PropertiesPanel {
    pub fn new(core: Entity<EditorCore>, selected_clip_id: Option<String>, active_right_tab: crate::ui::workspace::RightTab) -> Self {
        Self { core, selected_clip_id, active_right_tab }
    }

    pub fn render(self, cx: &mut Context<crate::ui::Workspace>) -> impl IntoElement {
        let workspace_handle = cx.entity().clone();
        let core = self.core.clone();
        let selected_clip_id = self.selected_clip_id.clone();

        // 1. Find the selected clip in the timeline
        let mut target_clip: Option<Clip> = None;
        if let Some(ref clip_id) = selected_clip_id {
            for track in &core.read(cx).timeline.tracks {
                if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id) {
                    target_clip = Some(clip.clone());
                    break;
                }
            }
        }

        let panel_body = if let Some(clip) = target_clip {
            let scale_val = clip.scale.unwrap_or(1.0);
            let rot_val = clip.rotation.unwrap_or(0.0);
            let px_val = clip.position_x.unwrap_or(0.0);
            let py_val = clip.position_y.unwrap_or(0.0);
            let op_val = clip.opacity.unwrap_or(1.0);

            let clip_id = clip.id.clone();
            let core_clone = core.clone();

            let is_text_clip = clip.clip_type.as_deref() == Some("text");

            let mut container = div()
                .id("properties-container")
                .flex_1()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .child(
                    // Selected Clip Info Header
                    div()
                        .border_b(px(1.))
                        .border_color(rgb(0x2d2d30))
                        .pb_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8e8e93))
                                .child("已選取剪輯")
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child(clip.name.clone())
                        )
                );

            let active_tab = self.active_right_tab;

            match active_tab {
                crate::ui::workspace::RightTab::Transform => {
                    container = container
                        // 1. Scale Control Group
                        .child(
                            self.render_control_group(
                                "縮放 (Scale)",
                                format!("{:.1}x", scale_val),
                                self.render_action_btn("scale-dec", "- 0.1", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.scale;
                                        let new_val = Some((old_val.unwrap_or(1.0) - 0.1).clamp(0.1, 3.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            old_val, new_val,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_action_btn("scale-inc", "+ 0.1", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.scale;
                                        let new_val = Some((old_val.unwrap_or(1.0) + 0.1).clamp(0.1, 3.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            old_val, new_val,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_reset_btn("scale-reset", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.scale;
                                        let new_val = None; // Reset to 1.0
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            old_val, new_val,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                })
                            )
                        )
                        // 2. Rotation Control Group
                        .child(
                            self.render_control_group(
                                "旋轉 (Rotation)",
                                format!("{:.0}°", rot_val),
                                self.render_action_btn("rotation-dec", "- 15°", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.rotation;
                                        let mut val = old_val.unwrap_or(0.0) - 15.0;
                                        if val < -180.0 { val += 360.0; }
                                        let new_val = Some(val);
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            old_val, new_val,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_action_btn("rotation-inc", "+ 15°", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.rotation;
                                        let mut val = old_val.unwrap_or(0.0) + 15.0;
                                        if val > 180.0 { val -= 360.0; }
                                        let new_val = Some(val);
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            old_val, new_val,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_reset_btn("rotation-reset", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.rotation;
                                        let new_val = None; // Reset to 0.0
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            old_val, new_val,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                })
                            )
                        )
                        // 3. Position X Control Group
                        .child(
                            self.render_control_group(
                                "水平位置 (Position X)",
                                format!("{:.0} px", px_val),
                                self.render_action_btn("posx-dec", "左移 50", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_x;
                                        let new_val = Some((old_val.unwrap_or(0.0) - 50.0).clamp(-1000.0, 1000.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            old_val, new_val,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_action_btn("posx-inc", "右移 50", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_x;
                                        let new_val = Some((old_val.unwrap_or(0.0) + 50.0).clamp(-1000.0, 1000.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            old_val, new_val,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_reset_btn("posx-reset", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_x;
                                        let new_val = None; // Reset to 0.0
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            old_val, new_val,
                                            clip.position_y, clip.position_y,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                })
                            )
                        )
                        // 4. Position Y Control Group
                        .child(
                            self.render_control_group(
                                "垂直位置 (Position Y)",
                                format!("{:.0} px", py_val),
                                self.render_action_btn("posy-dec", "下移 50", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_y;
                                        let new_val = Some((old_val.unwrap_or(0.0) + 50.0).clamp(-1000.0, 1000.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            old_val, new_val,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_action_btn("posy-inc", "上移 50", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_y;
                                        let new_val = Some((old_val.unwrap_or(0.0) - 50.0).clamp(-1000.0, 1000.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            old_val, new_val,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_reset_btn("posy-reset", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.position_y;
                                        let new_val = None; // Reset to 0.0
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            old_val, new_val,
                                            clip.opacity, clip.opacity,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                })
                            )
                        );
                }
                crate::ui::workspace::RightTab::Audio => {
                    if !is_text_clip {
                        let vol_val = clip.volume.unwrap_or(1.0);
                        let fade_in_val = clip.fade_in.unwrap_or(0.0);
                        let fade_out_val = clip.fade_out.unwrap_or(0.0);

                        container = container
                            // 1. Volume Control
                            .child(
                                self.render_control_group(
                                    "音量 (Volume)",
                                    format!("{:.0}%", vol_val * 100.0),
                                    self.render_action_btn("vol-dec", "- 10%", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_vol = clip.volume;
                                            let new_vol = Some((old_vol.unwrap_or(1.0) - 0.1).clamp(0.0, 2.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                old_vol, new_vol,
                                                clip.fade_in, clip.fade_in,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_action_btn("vol-inc", "+ 10%", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_vol = clip.volume;
                                            let new_vol = Some((old_vol.unwrap_or(1.0) + 0.1).clamp(0.0, 2.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                old_vol, new_vol,
                                                clip.fade_in, clip.fade_in,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_reset_btn("vol-reset", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_vol = clip.volume;
                                            let new_vol = None; // Reset to 1.0
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                old_vol, new_vol,
                                                clip.fade_in, clip.fade_in,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                            )
                            // 2. Fade In Control
                            .child(
                                self.render_control_group(
                                    "淡入時間 (Fade In)",
                                    format!("{:.1} s", fade_in_val),
                                    self.render_action_btn("fadein-dec", "- 0.5s", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_in;
                                            let new_fade = Some((old_fade.unwrap_or(0.0) - 0.5).clamp(0.0, 10.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                old_fade, new_fade,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_action_btn("fadein-inc", "+ 0.5s", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_in;
                                            let new_fade = Some((old_fade.unwrap_or(0.0) + 0.5).clamp(0.0, 10.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                old_fade, new_fade,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_reset_btn("fadein-reset", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_in;
                                            let new_fade = None; // Reset to 0.0
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                old_fade, new_fade,
                                                clip.fade_out, clip.fade_out,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                            )
                            // 3. Fade Out Control
                            .child(
                                self.render_control_group(
                                    "淡出時間 (Fade Out)",
                                    format!("{:.1} s", fade_out_val),
                                    self.render_action_btn("fadeout-dec", "- 0.5s", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_out;
                                            let new_fade = Some((old_fade.unwrap_or(0.0) - 0.5).clamp(0.0, 10.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                clip.fade_in, clip.fade_in,
                                                old_fade, new_fade,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_action_btn("fadeout-inc", "+ 0.5s", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_out;
                                            let new_fade = Some((old_fade.unwrap_or(0.0) + 0.5).clamp(0.0, 10.0));
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                clip.fade_in, clip.fade_in,
                                                old_fade, new_fade,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_reset_btn("fadeout-reset", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_fade = clip.fade_out;
                                            let new_fade = None; // Reset to 0.0
                                            let cmd = EditClipAudioCommand::new(
                                                clip_id.clone(),
                                                clip.volume, clip.volume,
                                                clip.fade_in, clip.fade_in,
                                                old_fade, new_fade,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                            );
                    } else {
                        container = container.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8e8e93))
                                .child("此片段無音訊屬性")
                        );
                    }
                }
                crate::ui::workspace::RightTab::Speed => {
                    container = container.child(
                        self.render_control_group(
                            "播放速度 (Speed - Mock)",
                            "1.0x".to_string(),
                            self.render_action_btn("speed-dec", "- 0.1x", |_| {}),
                            self.render_action_btn("speed-inc", "+ 0.1x", |_| {}),
                            self.render_reset_btn("speed-reset", |_| {})
                        )
                    );
                }
                crate::ui::workspace::RightTab::Blend => {
                    container = container
                        // Opacity Control Group
                        .child(
                            self.render_control_group(
                                "不透明度 (Opacity)",
                                format!("{:.0}%", op_val * 100.0),
                                self.render_action_btn("opacity-dec", "- 10%", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.opacity;
                                        let new_val = Some((old_val.unwrap_or(1.0) - 0.1).clamp(0.0, 1.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            old_val, new_val,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_action_btn("opacity-inc", "+ 10%", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.opacity;
                                        let new_val = Some((old_val.unwrap_or(1.0) + 0.1).clamp(0.0, 1.0));
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            old_val, new_val,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                }),
                                self.render_reset_btn("opacity-reset", {
                                    let core = core_clone.clone();
                                    let clip = clip.clone();
                                    let clip_id = clip_id.clone();
                                    move |cx| {
                                        let old_val = clip.opacity;
                                        let new_val = None; // Reset to 1.0
                                        let cmd = EditClipTransformCommand::new(
                                            clip_id.clone(),
                                            clip.scale, clip.scale,
                                            clip.rotation, clip.rotation,
                                            clip.position_x, clip.position_x,
                                            clip.position_y, clip.position_y,
                                            old_val, new_val,
                                        );
                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                    }
                                })
                            )
                        );

                    if !is_text_clip {
                        container = container
                            // 4. Blend Mode Control
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1_5()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0xc5c5c7))
                                                    .child("混合模式 (Blend Mode)")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x8b5cf6))
                                                    .child(match clip.blend_mode.as_deref() {
                                                        Some("multiply") => "正片疊底",
                                                        Some("screen") => "濾色",
                                                        Some("overlay") => "疊加",
                                                        _ => "正常",
                                                    })
                                            )
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(
                                                self.render_action_btn("blend-normal", "正常", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipBlendModeCommand::new(
                                                            clip_id.clone(),
                                                            clip.blend_mode.clone(),
                                                            None,
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("blend-multiply", "正片疊底", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipBlendModeCommand::new(
                                                            clip_id.clone(),
                                                            clip.blend_mode.clone(),
                                                            Some("multiply".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("blend-screen", "濾色", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipBlendModeCommand::new(
                                                            clip_id.clone(),
                                                            clip.blend_mode.clone(),
                                                            Some("screen".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("blend-overlay", "疊加", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipBlendModeCommand::new(
                                                            clip_id.clone(),
                                                            clip.blend_mode.clone(),
                                                            Some("overlay".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                    )
                            );
                    }
                }
                crate::ui::workspace::RightTab::Mask => {
                    container = container.child(
                        self.render_control_group(
                            "畫面裁切 (Crop Mask - Mock)",
                            "16:9".to_string(),
                            self.render_action_btn("mask-16-9", "16:9", |_| {}),
                            self.render_action_btn("mask-4-3", "4:3", |_| {}),
                            self.render_reset_btn("mask-reset", |_| {})
                        )
                    );
                }
                crate::ui::workspace::RightTab::Filters => {
                    container = container
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1_5()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0xc5c5c7))
                                                .child("套用濾鏡 (Filter)")
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0x8b5cf6))
                                                .child(clip.filter.clone().unwrap_or_else(|| "無".to_string()))
                                        )
                                )
                                .child(
                                    self.render_reset_btn("filter-reset", {
                                        let core = core_clone.clone();
                                        let clip_id = clip_id.clone();
                                        let clip = clip.clone();
                                        move |cx| {
                                            let cmd = crate::editor::ApplyEffectCommand::new(
                                                clip_id.clone(),
                                                clip.filter.clone(),
                                                clip.transition.clone(),
                                                None,
                                                clip.transition.clone(),
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1_5()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0xc5c5c7))
                                                .child("套用轉場 (Transition)")
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0x8b5cf6))
                                                .child(clip.transition.clone().unwrap_or_else(|| "無".to_string()))
                                        )
                                )
                                .child(
                                    self.render_reset_btn("transition-reset", {
                                        let core = core_clone.clone();
                                        let clip_id = clip_id.clone();
                                        let clip = clip.clone();
                                        move |cx| {
                                            let cmd = crate::editor::ApplyEffectCommand::new(
                                                clip_id.clone(),
                                                clip.filter.clone(),
                                                clip.transition.clone(),
                                                clip.filter.clone(),
                                                None,
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                        );
                }
                crate::ui::workspace::RightTab::Text => {
                    if is_text_clip {
                        let text_val = clip.text_content.clone().unwrap_or_else(|| "文字".to_string());
                        let font_size_val = clip.font_size.unwrap_or(28.0);
                        let color_val = clip.text_color.clone().unwrap_or_else(|| "#ffffff".to_string());

                        container = container
                            // 1. Text Content Control
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1_5()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0xc5c5c7))
                                                    .child("文字內容")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x8b5cf6))
                                                    .child(if text_val.chars().count() > 10 { 
                                                        format!("{}...", text_val.chars().take(8).collect::<String>()) 
                                                    } else { 
                                                        text_val.clone() 
                                                    })
                                            )
                                    )
                                    .child(
                                        // Custom Edit Button
                                        div()
                                            .id("edit-text-custom-btn")
                                            .w_full()
                                            .py_1_5()
                                            .px_3()
                                            .bg(rgb(0x8b5cf6))
                                            .hover(|s| s.bg(rgb(0x7c3aed)))
                                            .rounded(px(4.))
                                            .cursor_pointer()
                                            .on_click({
                                                let core = core_clone.clone();
                                                let clip_id = clip_id.clone();
                                                let clip = clip.clone();
                                                move |_, _, cx| {
                                                    let core = core.clone();
                                                    let clip_id = clip_id.clone();
                                                    let old_content = clip.text_content.clone();
                                                    let old_color = clip.text_color.clone();
                                                    let font_size = clip.font_size;
                                                    cx.spawn(move |cx: &mut gpui::AsyncApp| {
                                                        let mut cx = cx.clone();
                                                        async move {
                                                            let (tx, rx) = std::sync::mpsc::channel();
                                                            let prompt_text = old_content.clone().unwrap_or_default();
                                                            std::thread::spawn(move || {
                                                                let script = format!(
                                                                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                                                                     [System.Reflection.Assembly]::LoadWithPartialName('Microsoft.VisualBasic') | Out-Null; \
                                                                     $val = [Microsoft.VisualBasic.Interaction]::InputBox('請輸入字幕/文字內容：', '編輯文字', '{}'); \
                                                                     Write-Output $val",
                                                                    prompt_text.replace("'", "''")
                                                                );
                                                                let output = std::process::Command::new("powershell")
                                                                    .args(&["-NoProfile", "-Command", &script])
                                                                    .output();
                                                                if let Ok(output) = output {
                                                                    if output.status.success() {
                                                                        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                                                        let _ = tx.send(Some(text));
                                                                        return;
                                                                    }
                                                                }
                                                                let _ = tx.send(None);
                                                            });
                                                            if let Ok(Some(new_text)) = rx.recv() {
                                                                let has_changed = Some(new_text.clone()) != old_content;
                                                                if has_changed && !new_text.is_empty() {
                                                                    let _ = core.update(&mut cx, |core, cx| {
                                                                        let cmd = EditClipTextCommand::new(
                                                                            clip_id,
                                                                            old_content,
                                                                            Some(new_text),
                                                                            font_size,
                                                                            font_size,
                                                                            old_color.clone(),
                                                                            old_color,
                                                                        );
                                                                        core.execute_command(Box::new(cmd), cx);
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    }).detach();
                                                }
                                            })
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_align(gpui::TextAlign::Center)
                                                    .text_color(rgb(0xffffff))
                                                    .font_weight(FontWeight::BOLD)
                                                    .child("✍ 編輯自訂文字")
                                            )
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(
                                                self.render_action_btn("txt-c1", "標題 1", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            Some("歡迎來到 OpenCut！".to_string()),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            clip.text_color.clone(),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("txt-c2", "標題 2", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            Some("今日 Vibe: Rust coding".to_string()),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            clip.text_color.clone(),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("txt-c3", "標題 3", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            Some("感謝收看，請訂閱！".to_string()),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            clip.text_color.clone(),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_reset_btn("txt-reset", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            None,
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            clip.text_color.clone(),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                    )
                            )
                            // 2. Font Size Control
                            .child(
                                self.render_control_group(
                                    "字型大小 (Font Size)",
                                    format!("{:.0} px", font_size_val),
                                    self.render_action_btn("font-dec", "- 2 px", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_sz = clip.font_size;
                                            let new_sz = Some((old_sz.unwrap_or(28.0) - 2.0).clamp(12.0, 72.0));
                                            let cmd = EditClipTextCommand::new(
                                                clip_id.clone(),
                                                clip.text_content.clone(),
                                                clip.text_content.clone(),
                                                old_sz,
                                                new_sz,
                                                clip.text_color.clone(),
                                                clip.text_color.clone(),
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_action_btn("font-inc", "+ 2 px", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_sz = clip.font_size;
                                            let new_sz = Some((old_sz.unwrap_or(28.0) + 2.0).clamp(12.0, 72.0));
                                            let cmd = EditClipTextCommand::new(
                                                clip_id.clone(),
                                                clip.text_content.clone(),
                                                clip.text_content.clone(),
                                                old_sz,
                                                new_sz,
                                                clip.text_color.clone(),
                                                clip.text_color.clone(),
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    }),
                                    self.render_reset_btn("font-reset", {
                                        let core = core_clone.clone();
                                        let clip = clip.clone();
                                        let clip_id = clip_id.clone();
                                        move |cx| {
                                            let old_sz = clip.font_size;
                                            let new_sz = None; // Reset
                                            let cmd = EditClipTextCommand::new(
                                                clip_id.clone(),
                                                clip.text_content.clone(),
                                                clip.text_content.clone(),
                                                old_sz,
                                                new_sz,
                                                clip.text_color.clone(),
                                                clip.text_color.clone(),
                                            );
                                            let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                        }
                                    })
                                )
                            )
                            // 3. Text Color Control
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1_5()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0xc5c5c7))
                                                    .child("文字顏色")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x8b5cf6))
                                                    .child(color_val.clone())
                                            )
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(
                                                self.render_action_btn("color-w", "白色", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            clip.text_content.clone(),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            Some("#ffffff".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("color-y", "黃色", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            clip.text_content.clone(),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            Some("#facc15".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_action_btn("color-p", "粉色", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            clip.text_content.clone(),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            Some("#ec4899".to_string()),
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                            .child(
                                                self.render_reset_btn("color-reset", {
                                                    let core = core_clone.clone();
                                                    let clip = clip.clone();
                                                    let clip_id = clip_id.clone();
                                                    move |cx| {
                                                        let cmd = EditClipTextCommand::new(
                                                            clip_id.clone(),
                                                            clip.text_content.clone(),
                                                            clip.text_content.clone(),
                                                            clip.font_size,
                                                            clip.font_size,
                                                            clip.text_color.clone(),
                                                            None,
                                                        );
                                                        let _ = core.update(cx, |core, cx| core.execute_command(Box::new(cmd), cx));
                                                    }
                                                })
                                            )
                                    )
                            );
                    } else {
                        container = container.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8e8e93))
                                .child("此分頁僅適用於文字片段")
                        );
                    }
                }
            }

            container
        } else {
            div()
                .id("properties-empty")
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x8e8e93))
                        .text_align(TextAlign::Center)
                        .child("請在下方時間軸點選剪輯片段\n以進行畫面與音量屬性編輯。")
                )
        };

        div()
            .w(px(272.))
            .h_full()
            .bg(rgb(0x161618))
            .border_l(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .flex_col()
            .child(
                // Panel Header
                div()
                    .px_4()
                    .py_3()
                    .border_b(px(1.))
                    .border_color(rgb(0x242427))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("屬性面板")
                    )
                    .children(if selected_clip_id.is_some() {
                        let workspace_handle = workspace_handle.clone();
                        Some(
                            div()
                                .id("deselect-clip-btn")
                                .px_2()
                                .py(px(2.))
                                .bg(rgb(0x242427))
                                .rounded(px(3.))
                                .cursor_pointer()
                                .on_click(move |_, _, cx| {
                                    let _ = workspace_handle.update(cx, |workspace, cx| {
                                        workspace.selected_clip_id = None;
                                        cx.notify();
                                    });
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x8e8e93))
                                        .child("取消選取")
                                )
                        )
                    } else {
                        None
                    })
            )
            .child(panel_body)
    }

    fn render_control_group(&self, title: &'static str, value: String, btn_dec: impl IntoElement, btn_inc: impl IntoElement, btn_reset: impl IntoElement) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1_5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xc5c5c7))
                            .child(title)
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x8b5cf6))
                            .child(value)
                    )
            )
            .child(
                div()
                    .flex()
                    .gap_1_5()
                    .child(btn_dec)
                    .child(btn_inc)
                    .child(btn_reset)
            )
    }

    fn render_action_btn<F>(&self, id: &'static str, label: &'static str, action: F) -> impl IntoElement 
    where F: Fn(&mut gpui::App) + 'static + Send + Sync {
        div()
            .id(id)
            .flex_1()
            .py_1()
            .px_2()
            .bg(rgb(0x242427))
            .hover(|s| s.bg(rgb(0x2f2f33)))
            .rounded(px(4.))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                action(cx);
            })
            .child(
                div()
                    .text_xs()
                    .text_align(TextAlign::Center)
                    .text_color(rgb(0xffffff))
                    .child(label)
            )
    }

    fn render_reset_btn<F>(&self, id: &'static str, action: F) -> impl IntoElement 
    where F: Fn(&mut gpui::App) + 'static + Send + Sync {
        div()
            .id(id)
            .w(px(32.))
            .py_1()
            .bg(rgb(0x2a1a1a))
            .hover(|s| s.bg(rgb(0x3d2424)))
            .rounded(px(4.))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                action(cx);
            })
            .child(
                div()
                    .text_xs()
                    .text_align(TextAlign::Center)
                    .text_color(rgb(0xef4444))
                    .child("↩")
            )
    }
}
