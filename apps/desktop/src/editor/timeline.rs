use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub name: String,
    pub path: String,
    pub start: f64,
    pub duration: f64,
    pub color: String,
    pub filter: Option<String>,
    pub transition: Option<String>,
    pub scale: Option<f64>,
    pub rotation: Option<f64>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub opacity: Option<f64>,
    pub clip_type: Option<String>,
    pub text_content: Option<String>,
    pub font_size: Option<f32>,
    pub text_color: Option<String>,
    pub volume: Option<f32>,
    pub fade_in: Option<f64>,
    pub fade_out: Option<f64>,
    pub blend_mode: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub clips: Vec<Clip>,
    pub volume: Option<f64>,
}

pub struct TimelineManager {
    pub tracks: Vec<Track>,
}

impl TimelineManager {
    pub fn new() -> Self {
        // Pre-populate with three tracks and some mock clips for aesthetics
        let track_1 = Track {
            id: "track-1".to_string(),
            clips: vec![
                Clip {
                    id: "clip-1".to_string(),
                    name: "片頭影片.mp4".to_string(),
                    path: "".to_string(),
                    start: 2.0,
                    duration: 12.0,
                    color: "#6366f1".to_string(), // Indigo
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
                    color: "#8b5cf6".to_string(), // Violet
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
            ],
            volume: Some(1.0),
        };

        let track_2 = Track {
            id: "track-2".to_string(),
            clips: vec![
                Clip {
                    id: "clip-overlay".to_string(),
                    name: "疊加影片.mp4".to_string(),
                    path: "".to_string(),
                    start: 5.0,
                    duration: 6.0,
                    color: "#f43f5e".to_string(), // Rose
                    filter: None,
                    transition: None,
                    scale: None,
                    rotation: None,
                    position_x: None,
                    position_y: None,
                    opacity: Some(0.8), // 80% opacity
                    clip_type: None,
                    text_content: None,
                    font_size: None,
                    text_color: None,
                    volume: Some(0.0), // Mute overlay track audio
                    fade_in: Some(0.0),
                    fade_out: Some(0.0),
                    blend_mode: Some("screen".to_string()),
                },
            ],
            volume: Some(1.0),
        };

        let track_3 = Track {
            id: "track-3".to_string(),
            clips: vec![
                Clip {
                    id: "clip-3".to_string(),
                    name: "背景音樂.mp3".to_string(),
                    path: "".to_string(),
                    start: 0.0,
                    duration: 48.0,
                    color: "#10b981".to_string(), // Emerald
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
                    volume: Some(0.8), // Start at 80% volume
                    fade_in: Some(2.0),  // 2s fade in
                    fade_out: Some(3.0), // 3s fade out
                    blend_mode: None,
                },
            ],
            volume: Some(0.7), // Default background music volume slightly lower
        };

        Self {
            tracks: vec![track_1, track_2, track_3],
        }
    }

    #[allow(dead_code)]
    pub fn add_clip(&mut self, name: String, path: String, start: f64, duration: f64, color: String, track_index: usize) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        
        let new_clip = Clip {
            id: format!("clip-{}", timestamp),
            name,
            path,
            start,
            duration,
            color,
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

        if track_index < self.tracks.len() {
            self.tracks[track_index].clips.push(new_clip);
        }
    }

    pub fn find_clip_at_time(&self, track_index: usize, time: f64) -> Option<&Clip> {
        if track_index >= self.tracks.len() {
            return None;
        }
        let clips = &self.tracks[track_index].clips;
        
        // Use binary search as the fast path
        match clips.binary_search_by(|c| {
            if time < c.start {
                std::cmp::Ordering::Greater
            } else if time > c.start + c.duration {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(idx) => Some(&clips[idx]),
            Err(_) => {
                // Fallback to linear search in case of overlaps or slight unsortedness
                clips.iter().find(|c| time >= c.start && time <= c.start + c.duration)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_binary_search() {
        let manager = TimelineManager {
            tracks: vec![Track {
                id: "track-1".to_string(),
                clips: vec![
                    Clip {
                        id: "1".to_string(),
                        name: "clip1".to_string(),
                        path: "".to_string(),
                        start: 0.0,
                        duration: 5.0,
                        color: "".to_string(),
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
                    },
                    Clip {
                        id: "2".to_string(),
                        name: "clip2".to_string(),
                        path: "".to_string(),
                        start: 10.0,
                        duration: 5.0,
                        color: "".to_string(),
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
                    },
                ],
                volume: Some(1.0),
            }],
        };

        assert_eq!(manager.find_clip_at_time(0, 2.0).unwrap().id, "1");
        assert!(manager.find_clip_at_time(0, 6.0).is_none());
        assert_eq!(manager.find_clip_at_time(0, 12.0).unwrap().id, "2");
    }
}

