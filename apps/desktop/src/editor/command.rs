use gpui::Context;
use crate::editor::EditorCore;
use crate::editor::timeline::Clip;

pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>);
    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>);
}

pub struct CommandHistory {
    pub undo_stack: Vec<Box<dyn Command>>,
    pub redo_stack: Vec<Box<dyn Command>>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

pub struct AddClipCommand {
    clip: Option<Clip>,
    clip_id: String,
    name: String,
    path: String,
    start: f64,
    duration: f64,
    color: String,
    track_index: usize,
}

impl AddClipCommand {
    pub fn new(name: String, path: String, start: f64, duration: f64, color: String, track_index: usize) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let clip_id = format!("clip-{}", timestamp);

        Self {
            clip: None,
            clip_id,
            name,
            path,
            start,
            duration,
            color,
            track_index,
        }
    }
}

impl Command for AddClipCommand {
    fn name(&self) -> &'static str {
        "新增剪輯"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        println!("AddClipCommand::execute called! Clip name: {}, track: {}", self.name, self.track_index);
        if let Some(clip) = self.clip.take() {
            if self.track_index < core.timeline.tracks.len() {
                core.timeline.tracks[self.track_index].clips.push(clip);
                cx.notify();
            }
        } else {
            let new_clip = Clip {
                id: self.clip_id.clone(),
                name: self.name.clone(),
                path: self.path.clone(),
                start: self.start,
                duration: self.duration,
                color: self.color.clone(),
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
                volume: None,
                fade_in: None,
                fade_out: None,
                blend_mode: None,
            };
            if self.track_index < core.timeline.tracks.len() {
                core.timeline.tracks[self.track_index].clips.push(new_clip);
                cx.notify();
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        println!("AddClipCommand::undo called!");
        if self.track_index < core.timeline.tracks.len() {
            let clips = &mut core.timeline.tracks[self.track_index].clips;
            if let Some(pos) = clips.iter().position(|c| c.id == self.clip_id) {
                let removed_clip = clips.remove(pos);
                self.clip = Some(removed_clip);
                cx.notify();
            }
        }
    }
}

pub struct EditClipCommand {
    clip_id: String,
    old_start: f64,
    old_duration: f64,
    new_start: f64,
    new_duration: f64,
}

impl EditClipCommand {
    pub fn new(clip_id: String, old_start: f64, old_duration: f64, new_start: f64, new_duration: f64) -> Self {
        Self {
            clip_id,
            old_start,
            old_duration,
            new_start,
            new_duration,
        }
    }
}

impl Command for EditClipCommand {
    fn name(&self) -> &'static str {
        "編輯剪輯"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.start = self.new_start;
                clip.duration = self.new_duration;
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.start = self.old_start;
                clip.duration = self.old_duration;
                cx.notify();
                break;
            }
        }
    }
}

pub struct SplitClipCommand {
    clip_id: String,
    new_clip_id: String,
    playhead: f64,
    old_duration: f64,
}

impl SplitClipCommand {
    pub fn new(clip_id: String, new_clip_id: String, playhead: f64, old_duration: f64) -> Self {
        Self {
            clip_id,
            new_clip_id,
            playhead,
            old_duration,
        }
    }
}

impl Command for SplitClipCommand {
    fn name(&self) -> &'static str {
        "分割剪輯"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            let clips = &mut track.clips;
            if let Some(pos) = clips.iter().position(|c| c.id == self.clip_id) {
                let original_clip = clips[pos].clone();
                // Shrink original clip
                clips[pos].duration = self.playhead - original_clip.start;
                
                // Create second part
                let second_part = Clip {
                    id: self.new_clip_id.clone(),
                    name: format!("{} (後半段)", original_clip.name),
                    path: original_clip.path.clone(),
                    start: self.playhead,
                    duration: self.old_duration - (self.playhead - original_clip.start),
                    color: original_clip.color.clone(),
                    filter: original_clip.filter.clone(),
                    transition: original_clip.transition.clone(),
                    scale: original_clip.scale,
                    rotation: original_clip.rotation,
                    position_x: original_clip.position_x,
                    position_y: original_clip.position_y,
                    opacity: original_clip.opacity,
                    clip_type: original_clip.clip_type.clone(),
                    text_content: original_clip.text_content.clone(),
                    font_size: original_clip.font_size,
                    text_color: original_clip.text_color.clone(),
                    volume: original_clip.volume,
                    fade_in: original_clip.fade_in,
                    fade_out: original_clip.fade_out,
                    blend_mode: original_clip.blend_mode.clone(),
                };
                clips.insert(pos + 1, second_part);
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            let clips = &mut track.clips;
            
            let mut found = false;
            // Remove second part
            if let Some(pos) = clips.iter().position(|c| c.id == self.new_clip_id) {
                clips.remove(pos);
                found = true;
            }
            
            // Restore original clip duration
            if let Some(pos) = clips.iter().position(|c| c.id == self.clip_id) {
                clips[pos].duration = self.old_duration;
                found = true;
            }

            if found {
                cx.notify();
                break;
            }
        }
    }
}

pub struct ApplyEffectCommand {
    clip_id: String,
    old_filter: Option<String>,
    old_transition: Option<String>,
    new_filter: Option<String>,
    new_transition: Option<String>,
}

impl ApplyEffectCommand {
    pub fn new(
        clip_id: String,
        old_filter: Option<String>,
        old_transition: Option<String>,
        new_filter: Option<String>,
        new_transition: Option<String>,
    ) -> Self {
        Self {
            clip_id,
            old_filter,
            old_transition,
            new_filter,
            new_transition,
        }
    }
}

impl Command for ApplyEffectCommand {
    fn name(&self) -> &'static str {
        "套用特效"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.filter = self.new_filter.clone();
                clip.transition = self.new_transition.clone();
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.filter = self.old_filter.clone();
                clip.transition = self.old_transition.clone();
                cx.notify();
                break;
            }
        }
    }
}

pub struct EditClipTransformCommand {
    clip_id: String,
    old_scale: Option<f64>,
    new_scale: Option<f64>,
    old_rotation: Option<f64>,
    new_rotation: Option<f64>,
    old_position_x: Option<f64>,
    new_position_x: Option<f64>,
    old_position_y: Option<f64>,
    new_position_y: Option<f64>,
    old_opacity: Option<f64>,
    new_opacity: Option<f64>,
}

impl EditClipTransformCommand {
    pub fn new(
        clip_id: String,
        old_scale: Option<f64>,
        new_scale: Option<f64>,
        old_rotation: Option<f64>,
        new_rotation: Option<f64>,
        old_position_x: Option<f64>,
        new_position_x: Option<f64>,
        old_position_y: Option<f64>,
        new_position_y: Option<f64>,
        old_opacity: Option<f64>,
        new_opacity: Option<f64>,
    ) -> Self {
        Self {
            clip_id,
            old_scale,
            new_scale,
            old_rotation,
            new_rotation,
            old_position_x,
            new_position_x,
            old_position_y,
            new_position_y,
            old_opacity,
            new_opacity,
        }
    }
}

impl Command for EditClipTransformCommand {
    fn name(&self) -> &'static str {
        "調整剪輯屬性"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.scale = self.new_scale;
                clip.rotation = self.new_rotation;
                clip.position_x = self.new_position_x;
                clip.position_y = self.new_position_y;
                clip.opacity = self.new_opacity;
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.scale = self.old_scale;
                clip.rotation = self.old_rotation;
                clip.position_x = self.old_position_x;
                clip.position_y = self.old_position_y;
                clip.opacity = self.old_opacity;
                cx.notify();
                break;
            }
        }
    }
}

pub struct EditClipTextCommand {
    clip_id: String,
    old_content: Option<String>,
    new_content: Option<String>,
    old_font_size: Option<f32>,
    new_font_size: Option<f32>,
    old_color: Option<String>,
    new_color: Option<String>,
}

impl EditClipTextCommand {
    pub fn new(
        clip_id: String,
        old_content: Option<String>,
        new_content: Option<String>,
        old_font_size: Option<f32>,
        new_font_size: Option<f32>,
        old_color: Option<String>,
        new_color: Option<String>,
    ) -> Self {
        Self {
            clip_id,
            old_content,
            new_content,
            old_font_size,
            new_font_size,
            old_color,
            new_color,
        }
    }
}

impl Command for EditClipTextCommand {
    fn name(&self) -> &'static str {
        "編輯文字屬性"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.text_content = self.new_content.clone();
                clip.font_size = self.new_font_size;
                clip.text_color = self.new_color.clone();
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.text_content = self.old_content.clone();
                clip.font_size = self.old_font_size;
                clip.text_color = self.old_color.clone();
                cx.notify();
                break;
            }
        }
    }
}

pub struct EditClipAudioCommand {
    clip_id: String,
    old_volume: Option<f32>,
    new_volume: Option<f32>,
    old_fade_in: Option<f64>,
    new_fade_in: Option<f64>,
    old_fade_out: Option<f64>,
    new_fade_out: Option<f64>,
}

impl EditClipAudioCommand {
    pub fn new(
        clip_id: String,
        old_volume: Option<f32>,
        new_volume: Option<f32>,
        old_fade_in: Option<f64>,
        new_fade_in: Option<f64>,
        old_fade_out: Option<f64>,
        new_fade_out: Option<f64>,
    ) -> Self {
        Self {
            clip_id,
            old_volume,
            new_volume,
            old_fade_in,
            new_fade_in,
            old_fade_out,
            new_fade_out,
        }
    }
}

impl Command for EditClipAudioCommand {
    fn name(&self) -> &'static str {
        "編輯音訊屬性"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.volume = self.new_volume;
                clip.fade_in = self.new_fade_in;
                clip.fade_out = self.new_fade_out;
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.volume = self.old_volume;
                clip.fade_in = self.old_fade_in;
                clip.fade_out = self.old_fade_out;
                cx.notify();
                break;
            }
        }
    }
}

pub struct EditClipBlendModeCommand {
    clip_id: String,
    old_blend_mode: Option<String>,
    new_blend_mode: Option<String>,
}

impl EditClipBlendModeCommand {
    pub fn new(
        clip_id: String,
        old_blend_mode: Option<String>,
        new_blend_mode: Option<String>,
    ) -> Self {
        Self {
            clip_id,
            old_blend_mode,
            new_blend_mode,
        }
    }
}

impl Command for EditClipBlendModeCommand {
    fn name(&self) -> &'static str {
        "編輯混合模式"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.blend_mode = self.new_blend_mode.clone();
                cx.notify();
                break;
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        for track in &mut core.timeline.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                clip.blend_mode = self.old_blend_mode.clone();
                cx.notify();
                break;
            }
        }
    }
}

pub struct DeleteClipCommand {
    clip_id: String,
    track_index: Option<usize>,
    deleted_clip: Option<Clip>,
    deleted_pos: Option<usize>,
}

impl DeleteClipCommand {
    pub fn new(clip_id: String) -> Self {
        Self {
            clip_id,
            track_index: None,
            deleted_clip: None,
            deleted_pos: None,
        }
    }
}

impl Command for DeleteClipCommand {
    fn name(&self) -> &'static str {
        "刪除剪輯"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        let mut found = None;
        for (t_idx, track) in core.timeline.tracks.iter().enumerate() {
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.clip_id) {
                found = Some((t_idx, pos));
                break;
            }
        }
        if let Some((t_idx, pos)) = found {
            self.track_index = Some(t_idx);
            self.deleted_pos = Some(pos);
            let clip = core.timeline.tracks[t_idx].clips.remove(pos);
            self.deleted_clip = Some(clip);
            cx.notify();
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        if let (Some(t_idx), Some(pos), Some(clip)) = (self.track_index, self.deleted_pos, self.deleted_clip.take()) {
            core.timeline.tracks[t_idx].clips.insert(pos, clip);
            cx.notify();
        }
    }
}

pub struct AddTextClipCommand {
    clip: Option<Clip>,
    clip_id: String,
    name: String,
    content: String,
    start: f64,
    duration: f64,
    font_size: f64,
    color: String,
    track_index: usize,
}

impl AddTextClipCommand {
    pub fn new(
        name: String,
        content: String,
        start: f64,
        duration: f64,
        font_size: f64,
        color: String,
        track_index: usize,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let clip_id = format!("clip-text-{}", timestamp);
        Self {
            clip: None,
            clip_id,
            name,
            content,
            start,
            duration,
            font_size,
            color,
            track_index,
        }
    }
}

impl Command for AddTextClipCommand {
    fn name(&self) -> &'static str {
        "新增文字"
    }

    fn execute(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        if let Some(clip) = self.clip.take() {
            if self.track_index < core.timeline.tracks.len() {
                core.timeline.tracks[self.track_index].clips.push(clip);
                cx.notify();
            }
        } else {
            let new_clip = Clip {
                id: self.clip_id.clone(),
                name: self.name.clone(),
                path: "".to_string(),
                start: self.start,
                duration: self.duration,
                color: self.color.clone(),
                filter: None,
                transition: None,
                scale: Some(1.0),
                rotation: Some(0.0),
                position_x: Some(0.0),
                position_y: Some(120.0),
                opacity: Some(1.0),
                clip_type: Some("text".to_string()),
                text_content: Some(self.content.clone()),
                font_size: Some(self.font_size as f32),
                text_color: Some("#ffffff".to_string()),
                volume: None,
                fade_in: None,
                fade_out: None,
                blend_mode: None,
            };
            if self.track_index < core.timeline.tracks.len() {
                core.timeline.tracks[self.track_index].clips.push(new_clip);
                cx.notify();
            }
        }
    }

    fn undo(&mut self, core: &mut EditorCore, cx: &mut Context<EditorCore>) {
        if self.track_index < core.timeline.tracks.len() {
            let clips = &mut core.timeline.tracks[self.track_index].clips;
            if let Some(pos) = clips.iter().position(|c| c.id == self.clip_id) {
                let removed_clip = clips.remove(pos);
                self.clip = Some(removed_clip);
                cx.notify();
            }
        }
    }
}




