pub mod playback;
pub mod timeline;
pub mod command;
pub mod decoder;
pub mod project;

use gpui::Context;
pub use playback::{PlaybackManager, AudioHost};
pub use timeline::{TimelineManager, Clip};
pub use command::{Command, CommandHistory, AddClipCommand, EditClipCommand, SplitClipCommand, ApplyEffectCommand, EditClipTransformCommand, EditClipTextCommand, EditClipAudioCommand, EditClipBlendModeCommand, DeleteClipCommand, AddTextClipCommand};
pub use project::ProjectFile;

pub struct EditorCore {
    pub playback: PlaybackManager,
    pub timeline: TimelineManager,
    pub history: CommandHistory,
    pub current_frame: Option<std::sync::Arc<gpui::RenderImage>>,
    video_metadata: std::collections::HashMap<String, (u32, u32, f64)>,
    pub audio_waveforms: std::collections::HashMap<String, Vec<f32>>,
    last_update_time: std::time::Instant,
    pub frame_cache: std::sync::Arc<decoder::FrameCache>,
    active_decodes: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<(String, u32)>>>,
    pub audio_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<f32>>>>>,
    pub audio_host: Option<AudioHost>,
}

impl EditorCore {
    pub fn new() -> Self {
        Self {
            playback: PlaybackManager::new(),
            timeline: TimelineManager::new(),
            history: CommandHistory::new(),
            current_frame: None,
            video_metadata: std::collections::HashMap::new(),
            audio_waveforms: std::collections::HashMap::new(),
            last_update_time: std::time::Instant::now(),
            frame_cache: std::sync::Arc::new(decoder::FrameCache::new(30)),
            active_decodes: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            audio_cache: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            audio_host: None,
        }
    }

    pub fn load_audio_waveform(&mut self, path: String, cx: &mut Context<Self>) {
        if self.audio_waveforms.contains_key(&path) {
            return;
        }
        
        // Prevent double loading by inserting an empty vector
        self.audio_waveforms.insert(path.clone(), Vec::new());
        
        cx.spawn(move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let path_clone = path.clone();
                let samples = cx.background_executor().spawn(async move {
                    decoder::extract_audio_waveform(&path_clone)
                }).await;
                
                if let Some(data) = samples {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.audio_waveforms.insert(path, data);
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn load_audio_samples(&mut self, path: String, cx: &mut Context<Self>) {
        if path.is_empty() {
            return;
        }

        // Prevent double loading by checking cache
        {
            let cache = self.audio_cache.lock().unwrap();
            if cache.contains_key(&path) {
                return;
            }
        }

        let cache = self.audio_cache.clone();
        cx.spawn(move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let path_clone = path.clone();
                let samples = cx.background_executor().spawn(async move {
                    decoder::extract_audio_samples(&path_clone)
                }).await;

                if let Some(data) = samples {
                    let mut lock = cache.lock().unwrap();
                    lock.insert(path, std::sync::Arc::new(data));
                    
                    // Notify UI that audio data is ready
                    let _ = this.update(&mut cx, |_, cx| {
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        if self.playback.is_playing {
            return;
        }
        self.playback.is_playing = true;
        self.playback.is_playing_atomic.store(true, std::sync::atomic::Ordering::Relaxed);
        self.playback.atomic_time_ms.store((self.playback.current_time * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
        cx.notify();

        // Spawn AudioHost for audio mixer playback
        let tracks = std::sync::Arc::new(self.timeline.tracks.clone());
        let audio_cache = self.audio_cache.clone();
        let atomic_time_ms = self.playback.atomic_time_ms.clone();
        let is_playing_atomic = self.playback.is_playing_atomic.clone();
        let duration = self.playback.duration;

        self.audio_host = playback::AudioHost::new(
            tracks,
            audio_cache,
            atomic_time_ms,
            is_playing_atomic,
            duration,
        );

        let interval = std::time::Duration::from_millis(16);
        self.playback.play_task = Some(cx.spawn(move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(interval).await;
                    let ended: anyhow::Result<bool> = this.update(&mut cx, |this: &mut Self, cx: &mut Context<Self>| {
                        if !this.playback.is_playing {
                            return true;
                        }
                        
                        // Sync current playhead time with atomic audio thread time
                        let audio_ms = this.playback.atomic_time_ms.load(std::sync::atomic::Ordering::Relaxed);
                        this.playback.current_time = audio_ms as f64 / 1000.0;
                        
                        if this.playback.current_time >= this.playback.duration {
                            this.playback.current_time = this.playback.duration;
                            this.playback.is_playing = false;
                            this.playback.is_playing_atomic.store(false, std::sync::atomic::Ordering::Relaxed);
                            this.audio_host = None;
                            this.update_frame(cx);
                            cx.notify();
                            return true;
                        }
                        this.update_frame(cx);
                        this.prefetch_future_frames(cx);
                        cx.notify();
                        false
                    });
                    if ended.is_err() || ended.unwrap() {
                        break;
                    }
                }
            }
        }));
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.playback.is_playing = false;
        self.playback.is_playing_atomic.store(false, std::sync::atomic::Ordering::Relaxed);
        self.playback.play_task = None;
        self.audio_host = None; // Stop audio cpal stream
        cx.notify();
    }

    pub fn seek(&mut self, time: f64, cx: &mut Context<Self>) {
        self.playback.current_time = time.clamp(0.0, self.playback.duration);
        self.playback.atomic_time_ms.store((self.playback.current_time * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
        self.update_frame(cx);
        cx.notify();
    }

    pub fn execute_command(&mut self, mut command: Box<dyn Command>, cx: &mut Context<Self>) {
        command.execute(self, cx);
        self.history.undo_stack.push(command);
        self.history.redo_stack.clear();
        self.update_frame(cx);
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(mut command) = self.history.undo_stack.pop() {
            println!("撤銷指令: {}", command.name());
            command.undo(self, cx);
            self.history.redo_stack.push(command);
            self.update_frame(cx);
        } else {
            println!("沒有指令可以撤銷");
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(mut command) = self.history.redo_stack.pop() {
            println!("重做指令: {}", command.name());
            command.execute(self, cx);
            self.history.undo_stack.push(command);
            self.update_frame(cx);
        } else {
            println!("沒有指令可以重做");
        }
    }

    pub fn update_frame(&mut self, cx: &mut Context<Self>) {
        let current_time = self.playback.current_time;
        
        // 1. 收集所有軌道（限視訊軌 0 與 1）在 current_time 的 active Clips
        let mut active_clips = Vec::new();
        for track_idx in 0..=1 {
            if let Some(clip) = self.timeline.find_clip_at_time(track_idx, current_time) {
                let is_text = clip.clip_type.as_deref() == Some("text");
                if !is_text && !clip.path.is_empty() {
                    active_clips.push(clip.clone());
                }
            }
        }

        // 若無活動剪輯，清空畫面並通知 UI
        if active_clips.is_empty() {
            if self.current_frame.is_some() {
                self.current_frame = None;
                cx.notify();
            }
            return;
        }

        // Throttle updates during active playback to prevent overloading the CPU
        if self.playback.is_playing {
            let now = std::time::Instant::now();
            if now.duration_since(self.last_update_time).as_millis() < 30 {
                return;
            }
            self.last_update_time = now;
        }

        let scale_width = 640u32;
        let mut frames_to_blend = Vec::new();
        let mut all_cached = true;

        for clip in &active_clips {
            let path = clip.path.clone();
            let relative_time = current_time - clip.start;

            let meta = if let Some(meta) = self.video_metadata.get(&path) {
                Some(*meta)
            } else {
                if let Some(meta) = decoder::probe_video_metadata(&path) {
                    self.video_metadata.insert(path.clone(), meta);
                    Some(meta)
                } else {
                    None
                }
            };

            if let Some((width, height, _)) = meta {
                let scale_height = (((height as f64 * (scale_width as f64 / width as f64)) as u32) / 2) * 2;
                let scale_height = scale_height.max(2);

                if let Some(cached_raw_img) = self.frame_cache.get(&path, relative_time) {
                    frames_to_blend.push((clip.clone(), cached_raw_img, scale_height, width, height));
                } else {
                    all_cached = false;
                    self.trigger_async_decode(
                        path.clone(),
                        relative_time,
                        scale_width,
                        scale_height,
                        clip.clone(),
                        current_time,
                        width,
                        height,
                        cx,
                    );
                }
            } else {
                all_cached = false;
            }
        }

        if all_cached {
            self.apply_multi_track_blending(frames_to_blend, scale_width, cx);
            self.prefetch_future_frames(cx);
        }
    }

    fn apply_multi_track_blending(
        &mut self,
        frames: Vec<(timeline::Clip, std::sync::Arc<image::RgbaImage>, u32, u32, u32)>,
        scale_width: u32,
        cx: &mut Context<Self>,
    ) {
        if frames.is_empty() {
            return;
        }

        // 以底層（第一軌）影片的高度作為合成畫布的高度
        let scale_height = frames[0].2;
        
        let mut main_canvas = image::ImageBuffer::new(scale_width, scale_height);
        for pixel in main_canvas.pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        for (clip, cached_img, clip_scale_h, orig_w, orig_h) in frames {
            let mut clip_canvas = image::ImageBuffer::new(scale_width, scale_height);
            for pixel in clip_canvas.pixels_mut() {
                *pixel = image::Rgba([0, 0, 0, 0]);
            }

            self.render_single_clip_to_buffer(
                cached_img.as_ref().clone(),
                &clip,
                clip_scale_h,
                orig_w,
                orig_h,
                &mut clip_canvas,
            );

            let blend_mode = clip.blend_mode.as_deref().unwrap_or("normal");
            let clip_opacity = clip.opacity.unwrap_or(1.0) as f32;

            for y in 0..scale_height {
                for x in 0..scale_width {
                    let fg_pixel = clip_canvas.get_pixel(x, y);
                    let bg_pixel = main_canvas.get_pixel_mut(x, y);

                    if fg_pixel[3] == 0 {
                        continue;
                    }

                    self.blend_pixel(bg_pixel, fg_pixel, blend_mode, clip_opacity);
                }
            }
        }

        let frame = image::Frame::new(main_canvas);
        let render_image = gpui::RenderImage::new(smallvec::SmallVec::from_elem(frame, 1));
        self.current_frame = Some(std::sync::Arc::new(render_image));
        cx.notify();
    }

    fn render_single_clip_to_buffer(
        &self,
        mut buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        clip: &timeline::Clip,
        scale_height: u32,
        orig_width: u32,
        orig_height: u32,
        dest_canvas: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) {
        let scale_width = dest_canvas.width();
        let scale_canvas_h = dest_canvas.height();

        if let Some(ref filter) = clip.filter {
            match filter.as_str() {
                "grayscale" => {
                    for pixel in buffer.pixels_mut() {
                        let gray = (0.299 * pixel[0] as f64 + 0.587 * pixel[1] as f64 + 0.114 * pixel[2] as f64) as u8;
                        pixel[0] = gray;
                        pixel[1] = gray;
                        pixel[2] = gray;
                    }
                }
                "brighten" => {
                    for pixel in buffer.pixels_mut() {
                        pixel[0] = (pixel[0] as i16 + 20).clamp(0, 255) as u8;
                        pixel[1] = (pixel[1] as i16 + 20).clamp(0, 255) as u8;
                        pixel[2] = (pixel[2] as i16 + 20).clamp(0, 255) as u8;
                    }
                }
                "contrast" => {
                    let factor = 1.25;
                    for pixel in buffer.pixels_mut() {
                        pixel[0] = (((pixel[0] as f64 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                        pixel[1] = (((pixel[1] as f64 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                        pixel[2] = (((pixel[2] as f64 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                    }
                }
                _ => {}
            }
        }

        let s_val = clip.scale.unwrap_or(1.0);
        let r_val = clip.rotation.unwrap_or(0.0);
        let px_val = clip.position_x.unwrap_or(0.0);
        let py_val = clip.position_y.unwrap_or(0.0);

        let cx_dest = scale_width as f64 / 2.0;
        let cy_dest = scale_canvas_h as f64 / 2.0;
        let cx_src = scale_width as f64 / 2.0;
        let cy_src = scale_height as f64 / 2.0;

        let dx = px_val * (scale_width as f64 / orig_width as f64);
        let dy = py_val * (scale_canvas_h as f64 / orig_height as f64);

        let rad = r_val * std::f64::consts::PI / 180.0;
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        for y_dest in 0..scale_canvas_h {
            for x_dest in 0..scale_width {
                let x_prime = x_dest as f64 - cx_dest - dx;
                let y_prime = y_dest as f64 - cy_dest - dy;

                let x_double_prime = x_prime / s_val;
                let y_double_prime = y_prime / s_val;

                let x_src_coord = x_double_prime * cos_r + y_double_prime * sin_r + cx_src;
                let y_src_coord = -x_double_prime * sin_r + y_double_prime * cos_r + cy_src;

                let xs = x_src_coord.round() as i32;
                let ys = y_src_coord.round() as i32;

                if xs >= 0 && xs < scale_width as i32 && ys >= 0 && ys < scale_height as i32 {
                    if let Some(&orig_pixel) = buffer.get_pixel_checked(xs as u32, ys as u32) {
                        let dest_pixel = dest_canvas.get_pixel_mut(x_dest, y_dest);
                        *dest_pixel = orig_pixel;
                    }
                }
            }
        }
    }

    fn blend_pixel(&self, bg: &mut image::Rgba<u8>, fg: &image::Rgba<u8>, blend_mode: &str, clip_opacity: f32) {
        let fg_alpha = (fg[3] as f32 / 255.0) * clip_opacity;
        if fg_alpha <= 0.0 {
            return;
        }

        let bg_alpha = bg[3] as f32 / 255.0;

        let r_fg = fg[0] as f32 / 255.0;
        let g_fg = fg[1] as f32 / 255.0;
        let b_fg = fg[2] as f32 / 255.0;

        let r_bg = bg[0] as f32 / 255.0;
        let g_bg = bg[1] as f32 / 255.0;
        let b_bg = bg[2] as f32 / 255.0;

        let (r_blend, g_blend, b_blend) = match blend_mode {
            "multiply" => {
                (r_fg * r_bg, g_fg * g_bg, b_fg * b_bg)
            }
            "screen" => {
                (
                    r_fg + r_bg - r_fg * r_bg,
                    g_fg + g_bg - g_fg * g_bg,
                    b_fg + b_bg - b_fg * b_bg,
                )
            }
            "overlay" => {
                let overlay_ch = |cb: f32, cs: f32| -> f32 {
                    if cb < 0.5 {
                        2.0 * cs * cb
                    } else {
                        1.0 - 2.0 * (1.0 - cs) * (1.0 - cb)
                    }
                };
                (overlay_ch(r_bg, r_fg), overlay_ch(g_bg, g_fg), overlay_ch(b_bg, b_fg))
            }
            _ => {
                (r_fg, g_fg, b_fg)
            }
        };

        let out_alpha = fg_alpha + bg_alpha * (1.0 - fg_alpha);
        if out_alpha <= 0.0 {
            *bg = image::Rgba([0, 0, 0, 0]);
            return;
        }

        let blend_weight = fg_alpha / out_alpha;
        let bg_weight = bg_alpha * (1.0 - fg_alpha) / out_alpha;

        let r_out = r_blend * blend_weight + r_bg * bg_weight;
        let g_out = g_blend * blend_weight + g_bg * bg_weight;
        let b_out = b_blend * blend_weight + b_bg * bg_weight;

        bg[0] = (r_out * 255.0).clamp(0.0, 255.0) as u8;
        bg[1] = (g_out * 255.0).clamp(0.0, 255.0) as u8;
        bg[2] = (b_out * 255.0).clamp(0.0, 255.0) as u8;
        bg[3] = (out_alpha * 255.0).clamp(0.0, 255.0) as u8;
    }

    fn trigger_async_decode(
        &self,
        path: String,
        relative_time: f64,
        scale_width: u32,
        scale_height: u32,
        _clip: timeline::Clip,
        target_time: f64,
        _orig_width: u32,
        _orig_height: u32,
        cx: &mut Context<Self>,
    ) {
        let key = (path.clone(), (relative_time / 0.016).round() as u32);
        
        {
            let mut active = self.active_decodes.lock().unwrap();
            if active.contains(&key) {
                return;
            }
            active.insert(key.clone());
        }

        let cache = self.frame_cache.clone();
        let active_decodes = self.active_decodes.clone();

        cx.spawn(move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            let path = path.clone();
            let relative_time = relative_time;
            async move {
                let path_clone = path.clone();
                let raw_buffer: Option<image::RgbaImage> = cx.background_executor().spawn(async move {
                    let is_img = is_image_path(&path_clone);
                    let bgra_bytes = if is_img {
                        decoder::extract_static_image(&path_clone, scale_width, scale_height)
                    } else {
                        decoder::extract_video_frame(&path_clone, relative_time, scale_width, scale_height)
                    };

                    if let Some(bytes) = bgra_bytes {
                        image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(scale_width, scale_height, bytes)
                    } else {
                        None
                    }
                }).await;

                {
                    let mut active = active_decodes.lock().unwrap();
                    active.remove(&key);
                }

                if let Some(buf) = raw_buffer {
                    let buf_arc = std::sync::Arc::new(buf);
                    
                    cache.insert(&path, relative_time, buf_arc.clone());

                    let _ = this.update(&mut cx, |this: &mut Self, cx: &mut Context<Self>| {
                        let current_time = this.playback.current_time;
                        if (current_time - target_time).abs() < 0.05 {
                            this.update_frame(cx);
                        }
                    });
                }
            }
        }).detach();
    }

    fn prefetch_future_frames(&self, cx: &mut Context<Self>) {
        let current_time = self.playback.current_time;
        for track_idx in 0..=1 {
            if let Some(clip) = self.timeline.find_clip_at_time(track_idx, current_time).cloned() {
                if !clip.path.is_empty() {
                    for i in 1..=5 {
                        let prefetch_time = current_time + (i as f64) * 0.016 * self.playback.speed;
                        if prefetch_time > clip.start + clip.duration {
                            break;
                        }

                        let path = clip.path.clone();
                        let relative_time = prefetch_time - clip.start;
                        
                        let meta = self.video_metadata.get(&path).cloned();
                        if let Some((width, height, _)) = meta {
                            let scale_width = 640u32;
                            let scale_height = (((height as f64 * (scale_width as f64 / width as f64)) as u32) / 2) * 2;
                            let scale_height = scale_height.max(2);

                            if self.frame_cache.get(&path, relative_time).is_none() {
                                self.trigger_async_decode(
                                    path.clone(),
                                    relative_time,
                                    scale_width,
                                    scale_height,
                                    clip.clone(),
                                    prefetch_time,
                                    width,
                                    height,
                                    cx,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn split_clip_at_playhead(&mut self, cx: &mut Context<Self>) {
        let playhead = self.playback.current_time;
        
        let mut target_clip = None;
        if !self.timeline.tracks.is_empty() {
            for clip in &self.timeline.tracks[0].clips {
                if playhead > clip.start + 0.1 && playhead < clip.start + clip.duration - 0.1 {
                    target_clip = Some(clip.clone());
                    break;
                }
            }
        }

        if let Some(clip) = target_clip {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let new_clip_id = format!("clip-split-{}", timestamp);
            
            let cmd = SplitClipCommand::new(clip.id.clone(), new_clip_id, playhead, clip.duration);
            self.execute_command(Box::new(cmd), cx);
            println!("Split clip at playhead: {}s", playhead);
        } else {
            println!("No clip under playhead to split, or playhead too close to borders");
        }
    }

    pub fn new_project(&mut self, cx: &mut Context<Self>) {
        use timeline::Track;
        self.timeline.tracks = vec![
            Track { id: "track-1".to_string(), clips: vec![], volume: Some(1.0) },
            Track { id: "track-2".to_string(), clips: vec![], volume: Some(1.0) },
            Track { id: "track-3".to_string(), clips: vec![], volume: Some(1.0) },
            Track { id: "track-4".to_string(), clips: vec![], volume: Some(1.0) },
            Track { id: "track-5".to_string(), clips: vec![], volume: Some(1.0) },
        ];
        self.playback.duration = 60.0;
        self.playback.current_time = 0.0;
        self.history.undo_stack.clear();
        self.history.redo_stack.clear();
        self.current_frame = None;
        self.update_frame(cx);
        cx.notify();
        println!("New 5-track project initialized.");
    }

    pub fn save_project(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        let proj = ProjectFile {
            version: "1.0".to_string(),
            duration: self.playback.duration,
            tracks: self.timeline.tracks.clone(),
        };
        serde_json::to_writer_pretty(file, &proj)?;
        println!("Project saved to {:?}", path);
        Ok(())
    }

    pub fn load_project(&mut self, path: &std::path::Path, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let file = std::fs::File::open(path)?;
        let proj: ProjectFile = serde_json::from_reader(file)?;
        self.timeline.tracks = proj.tracks;
        self.playback.duration = proj.duration;
        self.playback.current_time = 0.0;
        self.history.undo_stack.clear();
        self.history.redo_stack.clear();
        self.current_frame = None;
        self.update_frame(cx);
        cx.notify();
        println!("Project loaded from {:?}", path);
        Ok(())
    }

    pub fn export_video(
        &self,
        export_path: &std::path::Path,
        status_updater: gpui::WeakEntity<crate::ui::Workspace>,
        cx: &mut Context<Self>,
    ) {
        // Collect Track 0 clips
        if self.timeline.tracks.is_empty() || self.timeline.tracks[0].clips.is_empty() {
            let _ = status_updater.update(cx, |workspace, cx| {
                workspace.export_state = crate::ui::workspace::ExportState::Failed("時間軸上沒有任何剪輯片段。".to_string());
                cx.notify();
            });
            return;
        }

        let clips = self.timeline.tracks[0].clips.clone();
        
        // Validate Track 0 paths exist
        for clip in &clips {
            if clip.path.is_empty() || !std::path::Path::new(&clip.path).exists() {
                let _ = status_updater.update(cx, |workspace, cx| {
                    workspace.export_state = crate::ui::workspace::ExportState::Failed(
                        format!("匯出失敗：找不到剪輯片段「{}」的實體影片路徑。請先點擊「+ 新增剪輯」加入實體影片，再進行匯出。", clip.name)
                    );
                    cx.notify();
                });
                return;
            }
        }

        // Collect Track 1 (Overlay) clips
        let mut overlay_clips = Vec::new();
        if self.timeline.tracks.len() > 1 {
            for clip in &self.timeline.tracks[1].clips {
                if !clip.path.is_empty() {
                    if !std::path::Path::new(&clip.path).exists() {
                        let _ = status_updater.update(cx, |workspace, cx| {
                            workspace.export_state = crate::ui::workspace::ExportState::Failed(
                                format!("匯出失敗：找不到疊加影片「{}」的實體影片路徑。請確定影片檔案存在，再進行匯出。", clip.name)
                            );
                            cx.notify();
                        });
                        return;
                    }
                    overlay_clips.push(clip.clone());
                }
            }
        }

        // Collect all non-primary tracks (Index 1 to end) clips for audio mixing
        let mut audio_clips = Vec::new();
        for track in self.timeline.tracks.iter().skip(1) {
            let track_vol = track.volume.unwrap_or(1.0);
            for clip in &track.clips {
                if !clip.path.is_empty() {
                    if !std::path::Path::new(&clip.path).exists() {
                        let _ = status_updater.update(cx, |workspace, cx| {
                            workspace.export_state = crate::ui::workspace::ExportState::Failed(
                                format!("匯出失敗：找不到音訊「{}」的實體音訊路徑。請確定音訊檔案存在，再進行匯出。", clip.name)
                            );
                            cx.notify();
                        });
                        return;
                    }
                    audio_clips.push((clip.clone(), track_vol));
                }
            }
        }

        let total_duration: f64 = clips.iter().map(|c| c.duration).sum();
        let export_path_buf = export_path.to_path_buf();
        let export_path_for_ffmpeg = export_path_buf.clone();
        let status_updater_for_ffmpeg = status_updater.clone();

        let track_0_vol = self.timeline.tracks.first().and_then(|t| t.volume).unwrap_or(1.0);

        // Spawn async background task to run FFmpeg and read progress
        cx.spawn(move |_, cx: &mut gpui::AsyncApp| {
            let cx = cx.clone();
            async move {
                let cx_for_ffmpeg = cx.clone();
                let result = move || -> anyhow::Result<()> {
                    let mut cmd = std::process::Command::new("ffmpeg");
                    cmd.arg("-y");
                    
                    // Add Track 0 (Video) inputs
                    for clip in &clips {
                        if is_image_path(&clip.path) {
                            cmd.arg("-loop").arg("1");
                            cmd.arg("-t").arg(format!("{:.3}", clip.duration));
                            cmd.arg("-i").arg(&clip.path);
                        } else {
                            cmd.arg("-t").arg(format!("{:.3}", clip.duration));
                            cmd.arg("-i").arg(&clip.path);
                        }
                    }

                    // Add Track 1 (Overlay Video) inputs
                    for clip in &overlay_clips {
                        if is_image_path(&clip.path) {
                            cmd.arg("-loop").arg("1");
                            cmd.arg("-t").arg(format!("{:.3}", clip.duration));
                            cmd.arg("-i").arg(&clip.path);
                        } else {
                            cmd.arg("-t").arg(format!("{:.3}", clip.duration));
                            cmd.arg("-i").arg(&clip.path);
                        }
                    }

                    // Add Audio inputs
                    for (clip, _) in &audio_clips {
                        cmd.arg("-t").arg(format!("{:.3}", clip.duration));
                        cmd.arg("-i").arg(&clip.path);
                    }

                    // Create filter graph
                    let mut filter_complex = String::new();
                    
                    // 1. Process each Video input (Track 0)
                    for (i, clip) in clips.iter().enumerate() {
                        let has_audio = decoder::has_audio_stream(&clip.path);
                        
                        // Video filter chain
                        let mut v_filter_chain = Vec::new();
                        if let Some(ref filter) = clip.filter {
                            match filter.as_str() {
                                "grayscale" => v_filter_chain.push("hue=s=0".to_string()),
                                "brighten" => v_filter_chain.push("eq=brightness=0.08".to_string()),
                                "contrast" => v_filter_chain.push("eq=contrast=1.25".to_string()),
                                _ => {}
                            }
                        }
                        if let Some(ref trans) = clip.transition {
                            if trans == "fade" && clip.duration > 1.0 {
                                v_filter_chain.push("fade=t=in:st=0:d=0.5".to_string());
                                v_filter_chain.push(format!("fade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
                            }
                        }
                        
                        // Opacity
                        let op = clip.opacity.unwrap_or(1.0);
                        if op < 1.0 {
                            v_filter_chain.push(format!("format=rgba,colorchannelmixer=aa={:.2}", op));
                        }

                        // Transform (Scale, Rotate, Position X/Y)
                        let scale_factor = clip.scale.unwrap_or(1.0);
                        let rot_deg = clip.rotation.unwrap_or(0.0);
                        let pos_x = clip.position_x.unwrap_or(0.0);
                        let pos_y = clip.position_y.unwrap_or(0.0);

                        let has_geom_transform = scale_factor != 1.0 || rot_deg != 0.0 || pos_x != 0.0 || pos_y != 0.0;
                        if has_geom_transform {
                            let (orig_w, orig_h) = if let Some(meta) = decoder::probe_video_metadata(&clip.path) {
                                (meta.0, meta.1)
                            } else {
                                (1920, 1080)
                            };

                            if scale_factor != 1.0 {
                                v_filter_chain.push(format!("scale=w=iw*{:.2}:h=ih*{:.2}", scale_factor, scale_factor));
                            }
                            if rot_deg != 0.0 {
                                v_filter_chain.push(format!("rotate={:.2}*PI/180:fillcolor=black", rot_deg));
                            }
                            v_filter_chain.push(format!(
                                "pad=w={}:h={}:x=(ow-iw)/2+{:.2}:y=(oh-ih)/2+{:.2}:color=black",
                                orig_w, orig_h, pos_x, pos_y
                            ));
                        }

                        if !v_filter_chain.is_empty() {
                            filter_complex.push_str(&format!("[{}:v]{}[v_proc_{}];", i, v_filter_chain.join(","), i));
                        }
                        
                        // Audio filter chain / silence generation
                        if has_audio {
                            let mut a_filter_chain = Vec::new();
                            if let Some(ref trans) = clip.transition {
                                if trans == "fade" && clip.duration > 1.0 {
                                    a_filter_chain.push("afade=t=in:st=0:d=0.5".to_string());
                                    a_filter_chain.push(format!("afade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
                                }
                            }
                            if !a_filter_chain.is_empty() {
                                filter_complex.push_str(&format!("[{}:a]{}[a_proc_{}];", i, a_filter_chain.join(","), i));
                            }
                        } else {
                            // Generate silent audio of the clip's duration
                            filter_complex.push_str(&format!("anullsrc=r=44100:cl=stereo,atrim=0:{:.3}[a_proc_{}];", clip.duration, i));
                        }
                    }

                    // 2. Concat processed Video & Audio inputs (Track 0)
                    for i in 0..clips.len() {
                        let clip = &clips[i];
                        let has_audio = decoder::has_audio_stream(&clip.path);
                        
                        let has_v_proc = clip.filter.is_some()
                            || (clip.transition.is_some() && clip.duration > 1.0)
                            || clip.opacity.unwrap_or(1.0) < 1.0
                            || clip.scale.unwrap_or(1.0) != 1.0
                            || clip.rotation.unwrap_or(0.0) != 0.0
                            || clip.position_x.unwrap_or(0.0) != 0.0
                            || clip.position_y.unwrap_or(0.0) != 0.0;

                        let v_label = if has_v_proc {
                            format!("[v_proc_{}]", i)
                        } else {
                            format!("[{}:v]", i)
                        };
                        
                        let a_label = if !has_audio || (clip.transition.is_some() && clip.duration > 1.0) {
                            format!("[a_proc_{}]", i)
                        } else {
                            format!("[{}:a]", i)
                        };
                        
                        filter_complex.push_str(&format!("{}{}", v_label, a_label));
                    }
                    filter_complex.push_str(&format!("concat=n={}:v=1:a=1[v_concat][a_concat];", clips.len()));

                    // Apply main track volume
                    filter_complex.push_str(&format!("[a_concat]volume={:.2}[a_main_vol]", track_0_vol));

                    // 3. Process Track 1 Overlay inputs
                    let n = clips.len();
                    let mut current_bg = "[v_concat]".to_string();
                    for (j, clip) in overlay_clips.iter().enumerate() {
                        let idx = n + j;
                        let mut v_filter_chain = Vec::new();
                        
                        if let Some(ref filter) = clip.filter {
                            match filter.as_str() {
                                "grayscale" => v_filter_chain.push("hue=s=0".to_string()),
                                "brighten" => v_filter_chain.push("eq=brightness=0.08".to_string()),
                                "contrast" => v_filter_chain.push("eq=contrast=1.25".to_string()),
                                _ => {}
                            }
                        }
                        
                        if let Some(ref trans) = clip.transition {
                            if trans == "fade" && clip.duration > 1.0 {
                                v_filter_chain.push("fade=t=in:st=0:d=0.5".to_string());
                                v_filter_chain.push(format!("fade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
                            }
                        }
                        
                        let op = clip.opacity.unwrap_or(1.0);
                        if op < 1.0 {
                            v_filter_chain.push(format!("format=rgba,colorchannelmixer=aa={:.2}", op));
                        }

                        let scale_factor = clip.scale.unwrap_or(1.0);
                        let rot_deg = clip.rotation.unwrap_or(0.0);
                        let pos_x = clip.position_x.unwrap_or(0.0);
                        let pos_y = clip.position_y.unwrap_or(0.0);

                        let has_geom_transform = scale_factor != 1.0 || rot_deg != 0.0 || pos_x != 0.0 || pos_y != 0.0;
                        if has_geom_transform {
                            let (orig_w, orig_h) = if let Some(meta) = decoder::probe_video_metadata(&clip.path) {
                                (meta.0, meta.1)
                            } else {
                                (1920, 1080)
                            };

                            if scale_factor != 1.0 {
                                v_filter_chain.push(format!("scale=w=iw*{:.2}:h=ih*{:.2}", scale_factor, scale_factor));
                            }
                            if rot_deg != 0.0 {
                                v_filter_chain.push(format!("rotate={:.2}*PI/180:fillcolor=black", rot_deg));
                            }
                            v_filter_chain.push(format!(
                                "pad=w={}:h={}:x=(ow-iw)/2+{:.2}:y=(oh-ih)/2+{:.2}:color=black@0",
                                orig_w, orig_h, pos_x, pos_y
                            ));
                        }

                        let v_label = if !v_filter_chain.is_empty() {
                            let label = format!("[v_overlay_proc_{}]", j);
                            filter_complex.push_str(&format!("[{}:v]{}{};", idx, v_filter_chain.join(","), label));
                            label
                        } else {
                            format!("[{}:v]", idx)
                        };

                        let next_bg = format!("[v_over_{}]", j);
                        let start = clip.start;
                        let end = clip.start + clip.duration;
                        
                        if !filter_complex.is_empty() && !filter_complex.ends_with(';') {
                            filter_complex.push(';');
                        }
                        filter_complex.push_str(&format!(
                            "{}{}overlay=x=0:y=0:enable='between(t,{:.3},{:.3})'{}",
                            current_bg, v_label, start, end, next_bg
                        ));
                        current_bg = next_bg;
                    }

                    // 4. Process all Audio inputs
                    let audio_start_idx = n + overlay_clips.len();
                    for (j, (clip, track_vol)) in audio_clips.iter().enumerate() {
                        let mut bgm_filter_chain = Vec::new();
                        if let Some(ref trans) = clip.transition {
                            if trans == "fade" && clip.duration > 1.0 {
                                bgm_filter_chain.push("afade=t=in:st=0:d=0.5".to_string());
                                bgm_filter_chain.push(format!("afade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
                            }
                        }
                        let combined_vol = (clip.volume.unwrap_or(1.0) as f64) * track_vol;
                        bgm_filter_chain.push(format!("volume={:.2}", combined_vol));
                        let delay_ms = (clip.start * 1000.0) as i64;
                        bgm_filter_chain.push(format!("adelay={}|{}", delay_ms, delay_ms));

                        if !filter_complex.is_empty() && !filter_complex.ends_with(';') {
                            filter_complex.push(';');
                        }
                        filter_complex.push_str(&format!(
                            "[{}:a]{}[delayed_bgm_{}]",
                            audio_start_idx + j,
                            bgm_filter_chain.join(","),
                            j
                        ));
                    }

                    // 5. Mix all audio tracks
                    if !audio_clips.is_empty() {
                        if !filter_complex.is_empty() && !filter_complex.ends_with(';') {
                            filter_complex.push(';');
                        }
                        filter_complex.push_str("[a_main_vol]");
                        for j in 0..audio_clips.len() {
                            filter_complex.push_str(&format!("[delayed_bgm_{}]", j));
                        }
                        filter_complex.push_str(&format!("amix=inputs={}:duration=first[a_mixed]", 1 + audio_clips.len()));
                    }

                    cmd.arg("-filter_complex").arg(filter_complex);
                    cmd.arg("-map").arg(&current_bg);
                    
                    if !audio_clips.is_empty() {
                        cmd.arg("-map").arg("[a_mixed]");
                    } else {
                        cmd.arg("-map").arg("[a_main_vol]");
                    }
                    cmd.arg(&export_path_for_ffmpeg);

                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::piped());

                    println!("Starting FFmpeg export: {:?}", cmd);
                    let mut child = cmd.spawn()?;
                    let stderr = child.stderr.take().ok_or_else(|| anyhow::anyhow!("Failed to open stderr of ffmpeg"))?;

                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(stderr);

                    for line_result in reader.lines() {
                        if let Ok(line) = line_result {
                            if let Some(pos) = line.find("time=") {
                                let time_part = &line[pos + 5..];
                                if time_part.len() >= 11 {
                                    let hms = &time_part[0..11];
                                    let parts: Vec<&str> = hms.split(':').collect();
                                    if parts.len() == 3 {
                                        if let (Ok(h), Ok(m), Ok(s)) = (
                                            parts[0].parse::<f64>(),
                                            parts[1].parse::<f64>(),
                                            parts[2].parse::<f64>(),
                                        ) {
                                            let current_time = h * 3600.0 + m * 60.0 + s;
                                            let progress = (current_time / total_duration).clamp(0.0, 0.99) as f32;
                                            
                                            let updater = status_updater_for_ffmpeg.clone();
                                            let cx_clone = cx_for_ffmpeg.clone();
                                            let _ = cx_clone.update(move |cx| {
                                                let _ = updater.update(cx, |workspace, cx| {
                                                    workspace.export_state = crate::ui::workspace::ExportState::Exporting { progress };
                                                    cx.notify();
                                                });
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let status = child.wait()?;
                    if !status.success() {
                        anyhow::bail!("FFmpeg export failed");
                    }

                    Ok(())
                }();

                let cx_final = cx.clone();
                let _ = cx_final.update(move |cx| {
                    let _ = status_updater.update(cx, |workspace, cx| {
                        match result {
                            Ok(_) => {
                                workspace.export_state = crate::ui::workspace::ExportState::Success { file_path: export_path_buf };
                            }
                            Err(e) => {
                                workspace.export_state = crate::ui::workspace::ExportState::Failed(format!("匯出失敗: {}", e));
                            }
                        }
                        cx.notify();
                    });
                });
            }
        }).detach();
    }
}

pub(crate) fn is_image_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_path() {
        assert!(is_image_path("photo.png"));
        assert!(is_image_path("image.JPG"));
        assert!(is_image_path("cool/pic.jpeg"));
        assert!(is_image_path("illustration.webp"));
        assert!(is_image_path("canvas.bmp"));
        assert!(!is_image_path("movie.mp4"));
        assert!(!is_image_path("song.mp3"));
        assert!(!is_image_path("text.txt"));
    }
}
