use crate::editor::timeline::Track;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gpui::Task;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioHost {
    _stream: cpal::Stream,
}

impl AudioHost {
    pub fn new(
        tracks: Arc<Vec<Track>>,
        audio_cache: Arc<Mutex<HashMap<String, Arc<Vec<f32>>>>>,
        atomic_time_ms: Arc<AtomicU64>,
        is_playing: Arc<AtomicBool>,
        duration: f64,
    ) -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        let stream_config = cpal::StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let err_fn = |err| println!("an error occurred on audio stream: {}", err);
        let sample_rate_val = sample_rate.0 as f64;
        let channels_count = channels as usize;

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !is_playing.load(Ordering::Relaxed) {
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                        return;
                    }

                    let current_time_ms = atomic_time_ms.load(Ordering::Relaxed);
                    let start_time = current_time_ms as f64 / 1000.0;

                    let cache_lock = audio_cache.lock().unwrap();
                    let num_frames = data.len() / channels_count;

                    // Reset buffer
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }

                    for f_idx in 0..num_frames {
                        let t_abs = start_time + (f_idx as f64 / sample_rate_val);
                        if t_abs >= duration {
                            break;
                        }

                        let mut mixed_l = 0.0f32;
                        let mut mixed_r = 0.0f32;

                        for track in tracks.iter() {
                            let track_vol = track.volume.unwrap_or(1.0) as f32;

                            for clip in &track.clips {
                                let is_text = clip.clip_type.as_deref() == Some("text");
                                if !is_text
                                    && t_abs >= clip.start
                                    && t_abs <= clip.start + clip.duration
                                {
                                    if let Some(pcm) = cache_lock.get(&clip.path) {
                                        let t_rel = t_abs - clip.start;
                                        let sample_idx = (t_rel * 44100.0).round() as usize;
                                        let pcm_idx_l = sample_idx * 2;
                                        let pcm_idx_r = sample_idx * 2 + 1;

                                        if pcm_idx_r < pcm.len() {
                                            let mut sample_l = pcm[pcm_idx_l];
                                            let mut sample_r = pcm[pcm_idx_r];

                                            let clip_vol = clip.volume.unwrap_or(1.0);
                                            sample_l *= clip_vol;
                                            sample_r *= clip_vol;

                                            // Fade In
                                            if let Some(fade_in_dur) = clip.fade_in {
                                                if fade_in_dur > 0.0 && t_rel < fade_in_dur {
                                                    let factor = (t_rel / fade_in_dur) as f32;
                                                    sample_l *= factor;
                                                    sample_r *= factor;
                                                }
                                            }

                                            // Fade Out
                                            let t_end_rel = clip.start + clip.duration - t_abs;
                                            if let Some(fade_out_dur) = clip.fade_out {
                                                if fade_out_dur > 0.0 && t_end_rel < fade_out_dur {
                                                    let factor = (t_end_rel / fade_out_dur) as f32;
                                                    sample_l *= factor;
                                                    sample_r *= factor;
                                                }
                                            }

                                            mixed_l += sample_l * track_vol;
                                            mixed_r += sample_r * track_vol;
                                        }
                                    }
                                }
                            }
                        }

                        if channels_count == 1 {
                            data[f_idx] = ((mixed_l + mixed_r) / 2.0).clamp(-1.0, 1.0);
                        } else {
                            data[f_idx * channels_count] = mixed_l.clamp(-1.0, 1.0);
                            data[f_idx * channels_count + 1] = mixed_r.clamp(-1.0, 1.0);
                            for c in 2..channels_count {
                                data[f_idx * channels_count + c] = 0.0;
                            }
                        }
                    }

                    let end_time = start_time + (num_frames as f64 / sample_rate_val);
                    atomic_time_ms.store((end_time * 1000.0) as u64, Ordering::Relaxed);
                },
                err_fn,
                None,
            )
            .ok()?;

        stream.play().ok()?;
        Some(Self { _stream: stream })
    }
}

pub struct PlaybackManager {
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub speed: f64,
    pub play_task: Option<Task<()>>,
    pub atomic_time_ms: Arc<AtomicU64>,
    pub is_playing_atomic: Arc<AtomicBool>,
}

impl PlaybackManager {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_time: 0.0,
            duration: 60.0, // Default 60 seconds project
            speed: 1.0,
            play_task: None,
            atomic_time_ms: Arc::new(AtomicU64::new(0)),
            is_playing_atomic: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::editor::EditorCore;
    use gpui::AppContext;

    #[gpui::test]
    async fn test_playback_controls(cx: &mut gpui::TestAppContext) {
        let core = cx.new(|_| EditorCore::demo());

        // 1. Initial State
        assert!(!cx.read(|cx| core.read(cx).playback.is_playing));
        assert_eq!(cx.read(|cx| core.read(cx).playback.current_time), 0.0);

        // 2. Play
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.play(cx);
            },
        );
        assert!(cx.read(|cx| core.read(cx).playback.is_playing));

        // 3. Pause
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.pause(cx);
            },
        );
        assert!(!cx.read(|cx| core.read(cx).playback.is_playing));

        // 4. Seek
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.seek(10.0, cx);
            },
        );
        assert_eq!(cx.read(|cx| core.read(cx).playback.current_time), 10.0);

        // 5. Seek beyond duration
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.seek(100.0, cx);
            },
        );
        assert_eq!(cx.read(|cx| core.read(cx).playback.current_time), 60.0); // Clamped to duration
    }

    #[gpui::test]
    async fn test_command_undo_redo(cx: &mut gpui::TestAppContext) {
        use crate::editor::AddClipCommand;
        use crate::editor::Clip;

        let core = cx.new(|_| EditorCore::new());
        let _ = core.update(cx, |core, _| {
            core.timeline.tracks[0].clips = vec![
                Clip {
                    id: "clip-1".to_string(),
                    name: "片頭影片.mp4".to_string(),
                    path: "".to_string(),
                    start: 2.0,
                    duration: 12.0,
                    color: "#6366f1".to_string(),
                    filter: None,
                    transition: None,
                    scale: None,
                    rotation: None,
                    position_x: None,
                    position_y: None,
                    opacity: None,
                    clip_type: None,
                    text_content: None,
                    font_size: None,
                    text_color: None,
                    volume: Some(1.0),
                    fade_in: Some(0.0),
                    fade_out: Some(0.0),
                    blend_mode: None,
                },
                Clip {
                    id: "clip-2".to_string(),
                    name: "日常Vlog_第一部分.mp4".to_string(),
                    path: "".to_string(),
                    start: 18.0,
                    duration: 25.0,
                    color: "#8b5cf6".to_string(),
                    filter: None,
                    transition: None,
                    scale: None,
                    rotation: None,
                    position_x: None,
                    position_y: None,
                    opacity: None,
                    clip_type: None,
                    text_content: None,
                    font_size: None,
                    text_color: None,
                    volume: Some(1.0),
                    fade_in: Some(0.0),
                    fade_out: Some(0.0),
                    blend_mode: None,
                },
            ];
        });

        // 1. Initial State: Track 0 has 2 clips
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            2
        );

        // 2. Execute AddClipCommand
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(AddClipCommand::new(
                        "TestClip.mp4".to_string(),
                        "".to_string(),
                        10.0,
                        5.0,
                        "#000000".to_string(),
                        0,
                    )),
                    cx,
                );
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            3
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[2].name.clone()),
            "TestClip.mp4"
        );

        // 3. Undo Command
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            2
        );

        // 4. Redo Command
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            3
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[2].name.clone()),
            "TestClip.mp4"
        );
    }

    #[gpui::test]
    async fn test_decoder_handling(cx: &mut gpui::TestAppContext) {
        use crate::editor::AddClipCommand;

        let core = cx.new(|_| EditorCore::demo());

        // 1. Initially, current_frame is None
        assert!(cx.read(|cx| core.read(cx).current_frame.is_none()));

        // 2. Seek to a mock clip time, current_frame should still be None
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.seek(5.0, cx);
            },
        );
        assert!(cx.read(|cx| core.read(cx).current_frame.is_none()));

        // 3. Add a clip with a non-existent path
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(AddClipCommand::new(
                        "NonExistent.mp4".to_string(),
                        "C:\\invalid_path_to_video.mp4".to_string(),
                        10.0,
                        5.0,
                        "#000000".to_string(),
                        0,
                    )),
                    cx,
                );
            },
        );

        // Seek to the invalid clip's start time
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.seek(11.0, cx);
            },
        );
        // It should handle the non-existent file gracefully without panicking and keep current_frame as None
        assert!(cx.read(|cx| core.read(cx).current_frame.is_none()));
    }

    #[gpui::test]
    async fn test_clip_edit_and_split(cx: &mut gpui::TestAppContext) {
        use crate::editor::Clip;
        use crate::editor::EditClipCommand;

        let core = cx.new(|_| EditorCore::new());
        let _ = core.update(cx, |core, _| {
            core.timeline.tracks[0].clips = vec![
                Clip {
                    id: "clip-1".to_string(),
                    name: "片頭影片.mp4".to_string(),
                    path: "".to_string(),
                    start: 2.0,
                    duration: 12.0,
                    color: "#6366f1".to_string(),
                    filter: None,
                    transition: None,
                    scale: None,
                    rotation: None,
                    position_x: None,
                    position_y: None,
                    opacity: None,
                    clip_type: None,
                    text_content: None,
                    font_size: None,
                    text_color: None,
                    volume: Some(1.0),
                    fade_in: Some(0.0),
                    fade_out: Some(0.0),
                    blend_mode: None,
                },
                Clip {
                    id: "clip-2".to_string(),
                    name: "日常Vlog_第一部分.mp4".to_string(),
                    path: "".to_string(),
                    start: 18.0,
                    duration: 25.0,
                    color: "#8b5cf6".to_string(),
                    filter: None,
                    transition: None,
                    scale: None,
                    rotation: None,
                    position_x: None,
                    position_y: None,
                    opacity: None,
                    clip_type: None,
                    text_content: None,
                    font_size: None,
                    text_color: None,
                    volume: Some(1.0),
                    fade_in: Some(0.0),
                    fade_out: Some(0.0),
                    blend_mode: None,
                },
            ];
        });

        // 1. Initially Track 0 has 2 clips
        let clips_len = cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len());
        assert_eq!(clips_len, 2);

        let clip0 = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        let clip0_id = clip0.id.clone();
        let clip0_start = clip0.start;
        let clip0_duration = clip0.duration;

        // 2. Test EditClipCommand (Move / Trim)
        let new_start = clip0_start + 2.0;
        let new_duration = clip0_duration - 1.0;
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(EditClipCommand::new(
                        clip0_id.clone(),
                        clip0_start,
                        clip0_duration,
                        new_start,
                        new_duration,
                    )),
                    cx,
                );
            },
        );

        // Verify edited clip values
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].start),
            new_start
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].duration),
            new_duration
        );

        // Undo Edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].start),
            clip0_start
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].duration),
            clip0_duration
        );

        // Redo Edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].start),
            new_start
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].duration),
            new_duration
        );

        // Reset state by undoing edit so we can test split cleanly on initial clip
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );

        // 3. Test SplitClipCommand
        // Playhead at clip0_start + 4.0
        let split_playhead = clip0_start + 4.0;
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.seek(split_playhead, cx);
                core.split_clip_at_playhead(cx);
            },
        );

        // Now we should have 3 clips on Track 0
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            3
        );

        // Verify original clip is shrunk
        let shrunk_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(shrunk_clip.duration, split_playhead - clip0_start);

        // Verify new split clip is inserted next
        let split_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].clone());
        assert_eq!(split_clip.start, split_playhead);
        assert_eq!(
            split_clip.duration,
            clip0_duration - (split_playhead - clip0_start)
        );

        // Undo Split
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            2
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].duration),
            clip0_duration
        );

        // Redo Split
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips.len()),
            3
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].duration),
            split_playhead - clip0_start
        );
        assert_eq!(
            cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].duration),
            clip0_duration - (split_playhead - clip0_start)
        );
    }

    #[gpui::test]
    async fn test_clip_transform_command(cx: &mut gpui::TestAppContext) {
        use crate::editor::EditClipTransformCommand;

        let core = cx.new(|_| EditorCore::demo());

        // Get clip-1
        let clip_id = "clip-1".to_string();
        let initial_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(initial_clip.id, clip_id);
        assert!(initial_clip.scale.is_none());

        // Execute transform edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(EditClipTransformCommand::new(
                        clip_id.clone(),
                        None,
                        Some(1.5),
                        None,
                        Some(45.0),
                        None,
                        Some(100.0),
                        None,
                        Some(-50.0),
                        None,
                        Some(0.8),
                    )),
                    cx,
                );
            },
        );

        // Verify edited values
        let edited = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(edited.scale, Some(1.5));
        assert_eq!(edited.rotation, Some(45.0));
        assert_eq!(edited.position_x, Some(100.0));
        assert_eq!(edited.position_y, Some(-50.0));
        assert_eq!(edited.opacity, Some(0.8));

        // Undo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        let reverted = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert!(reverted.scale.is_none());
        assert!(reverted.rotation.is_none());
        assert!(reverted.position_x.is_none());
        assert!(reverted.position_y.is_none());
        assert!(reverted.opacity.is_none());

        // Redo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        let redone = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(redone.scale, Some(1.5));
        assert_eq!(redone.rotation, Some(45.0));
        assert_eq!(redone.position_x, Some(100.0));
        assert_eq!(redone.position_y, Some(-50.0));
        assert_eq!(redone.opacity, Some(0.8));
    }

    #[gpui::test]
    async fn test_clip_text_command(cx: &mut gpui::TestAppContext) {
        use crate::editor::EditClipTextCommand;

        let core = cx.new(|_| EditorCore::demo());

        // Get clip-text-1 (index 1 of track 0)
        let clip_id = "clip-text-1".to_string();
        let initial_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].clone());
        assert_eq!(initial_clip.id, clip_id);
        assert_eq!(
            initial_clip.text_content,
            Some("歡迎使用 OpenCut 本地剪輯器！".to_string())
        );
        assert_eq!(initial_clip.font_size, Some(28.0));
        assert_eq!(initial_clip.text_color, Some("#ffffff".to_string()));

        // Execute text edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(EditClipTextCommand::new(
                        clip_id.clone(),
                        initial_clip.text_content.clone(),
                        Some("Hello World!".to_string()),
                        initial_clip.font_size,
                        Some(36.0),
                        initial_clip.text_color.clone(),
                        Some("#ff0000".to_string()),
                    )),
                    cx,
                );
            },
        );

        // Verify edited values
        let edited = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].clone());
        assert_eq!(edited.text_content, Some("Hello World!".to_string()));
        assert_eq!(edited.font_size, Some(36.0));
        assert_eq!(edited.text_color, Some("#ff0000".to_string()));

        // Undo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        let reverted = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].clone());
        assert_eq!(
            reverted.text_content,
            Some("歡迎使用 OpenCut 本地剪輯器！".to_string())
        );
        assert_eq!(reverted.font_size, Some(28.0));
        assert_eq!(reverted.text_color, Some("#ffffff".to_string()));

        // Redo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        let redone = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[1].clone());
        assert_eq!(redone.text_content, Some("Hello World!".to_string()));
        assert_eq!(redone.font_size, Some(36.0));
        assert_eq!(redone.text_color, Some("#ff0000".to_string()));
    }

    #[gpui::test]
    async fn test_edit_clip_audio_command(cx: &mut gpui::TestAppContext) {
        use crate::editor::EditClipAudioCommand;

        let core = cx.new(|_| EditorCore::demo());

        // Get clip-1
        let clip_id = "clip-1".to_string();
        let initial_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(initial_clip.id, clip_id);
        assert_eq!(initial_clip.volume, Some(1.0));
        assert_eq!(initial_clip.fade_in, Some(0.0));
        assert_eq!(initial_clip.fade_out, Some(0.0));

        // Execute audio property edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(EditClipAudioCommand::new(
                        clip_id.clone(),
                        initial_clip.volume,
                        Some(0.5),
                        initial_clip.fade_in,
                        Some(1.5),
                        initial_clip.fade_out,
                        Some(2.0),
                    )),
                    cx,
                );
            },
        );

        // Verify edited values
        let edited = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(edited.volume, Some(0.5));
        assert_eq!(edited.fade_in, Some(1.5));
        assert_eq!(edited.fade_out, Some(2.0));

        // Undo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        let reverted = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(reverted.volume, Some(1.0));
        assert_eq!(reverted.fade_in, Some(0.0));
        assert_eq!(reverted.fade_out, Some(0.0));

        // Redo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        let redone = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(redone.volume, Some(0.5));
        assert_eq!(redone.fade_in, Some(1.5));
        assert_eq!(redone.fade_out, Some(2.0));
    }

    #[gpui::test]
    async fn test_edit_clip_blend_mode_command(cx: &mut gpui::TestAppContext) {
        use crate::editor::EditClipBlendModeCommand;

        let core = cx.new(|_| EditorCore::demo());

        // Get clip-1
        let clip_id = "clip-1".to_string();
        let initial_clip = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(initial_clip.id, clip_id);
        assert!(initial_clip.blend_mode.is_none());

        // Execute blend mode edit
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.execute_command(
                    Box::new(EditClipBlendModeCommand::new(
                        clip_id.clone(),
                        initial_clip.blend_mode.clone(),
                        Some("multiply".to_string()),
                    )),
                    cx,
                );
            },
        );

        // Verify edited values
        let edited = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(edited.blend_mode, Some("multiply".to_string()));

        // Undo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.undo(cx);
            },
        );
        let reverted = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert!(reverted.blend_mode.is_none());

        // Redo
        let _ = core.update(
            cx,
            |core: &mut EditorCore, cx: &mut gpui::Context<EditorCore>| {
                core.redo(cx);
            },
        );
        let redone = cx.read(|cx| core.read(cx).timeline.tracks[0].clips[0].clone());
        assert_eq!(redone.blend_mode, Some("multiply".to_string()));
    }
}
