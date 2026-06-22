use serde::{Serialize, Deserialize};
use crate::editor::timeline::Track;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectFile {
    pub version: String,
    pub duration: f64,
    pub tracks: Vec<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::timeline::Clip;
    use std::fs::File;

    #[test]
    fn test_project_serialization() {
        let temp_path = std::env::temp_dir().join("test_project_serialization.opencut");

        let clip1 = Clip {
            id: "clip-1".to_string(),
            name: "test1.mp4".to_string(),
            path: "/path/to/test1.mp4".to_string(),
            start: 2.0,
            duration: 10.0,
            color: "#ff0000".to_string(),
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

        let track1 = Track {
            id: "track-1".to_string(),
            clips: vec![clip1],
            volume: Some(1.0),
        };

        let proj = ProjectFile {
            version: "1.0".to_string(),
            duration: 60.0,
            tracks: vec![track1],
        };

        // Serialize
        {
            let file = File::create(&temp_path).expect("Failed to create temp project file");
            serde_json::to_writer_pretty(file, &proj).expect("Failed to serialize project");
        }

        // Deserialize & Verify
        {
            let file = File::open(&temp_path).expect("Failed to open temp project file");
            let loaded: ProjectFile = serde_json::from_reader(file).expect("Failed to deserialize project");

            assert_eq!(loaded.version, "1.0");
            assert_eq!(loaded.duration, 60.0);
            assert_eq!(loaded.tracks.len(), 1);
            assert_eq!(loaded.tracks[0].clips.len(), 1);
            assert_eq!(loaded.tracks[0].clips[0].name, "test1.mp4");
            assert_eq!(loaded.tracks[0].clips[0].path, "/path/to/test1.mp4");
        }

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }
}
