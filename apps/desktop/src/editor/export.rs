use crate::editor::timeline::Clip;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ExportClip {
    pub clip: Clip,
    pub has_audio: bool,
    pub width: u32,
    pub height: u32,
    pub is_image: bool,
}

pub struct ExportPlan {
    pub clips: Vec<ExportClip>,
    pub overlay_clips: Vec<ExportClip>,
    pub audio_clips: Vec<(ExportClip, f64)>, // (clip, track_volume)
    pub track_0_vol: f64,
    pub export_path: PathBuf,
}

pub struct FfmpegCommandBuilder;

impl FfmpegCommandBuilder {
    pub fn build(plan: &ExportPlan) -> (std::process::Command, f64) {
        #[cfg(target_os = "windows")]
        let cmd_name = "ffmpeg.exe";
        #[cfg(not(target_os = "windows"))]
        let cmd_name = "ffmpeg";

        let mut cmd = std::process::Command::new(cmd_name);
        cmd.arg("-y");

        // 1. Add Track 0 (Video) inputs
        for ex_clip in &plan.clips {
            if ex_clip.is_image {
                cmd.arg("-loop").arg("1");
                cmd.arg("-t").arg(format!("{:.3}", ex_clip.clip.duration));
                cmd.arg("-i").arg(&ex_clip.clip.path);
            } else {
                cmd.arg("-t").arg(format!("{:.3}", ex_clip.clip.duration));
                cmd.arg("-i").arg(&ex_clip.clip.path);
            }
        }

        // 2. Add Track 1 (Overlay Video) inputs
        for ex_clip in &plan.overlay_clips {
            if ex_clip.is_image {
                cmd.arg("-loop").arg("1");
                cmd.arg("-t").arg(format!("{:.3}", ex_clip.clip.duration));
                cmd.arg("-i").arg(&ex_clip.clip.path);
            } else {
                cmd.arg("-t").arg(format!("{:.3}", ex_clip.clip.duration));
                cmd.arg("-i").arg(&ex_clip.clip.path);
            }
        }

        // 3. Add Audio inputs
        for (ex_clip, _) in &plan.audio_clips {
            cmd.arg("-t").arg(format!("{:.3}", ex_clip.clip.duration));
            cmd.arg("-i").arg(&ex_clip.clip.path);
        }

        // Calculate total duration of the main track
        let total_duration: f64 = plan.clips.iter().map(|c| c.clip.duration).sum();

        // Create filter graph
        let mut filter_complex = String::new();

        // 1. Process each Video input (Track 0)
        for (i, ex_clip) in plan.clips.iter().enumerate() {
            let clip = &ex_clip.clip;
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

            let has_geom_transform =
                scale_factor != 1.0 || rot_deg != 0.0 || pos_x != 0.0 || pos_y != 0.0;
            if has_geom_transform {
                let orig_w = ex_clip.width;
                let orig_h = ex_clip.height;

                if scale_factor != 1.0 {
                    v_filter_chain.push(format!(
                        "scale=w=iw*{:.2}:h=ih*{:.2}",
                        scale_factor, scale_factor
                    ));
                }
                if rot_deg != 0.0 {
                    v_filter_chain.push(format!("rotate={:.2}*PI/180:fillcolor=black", rot_deg));
                }
                v_filter_chain.push(format!(
                    "pad=w={}:h={}:x=(ow-iw)/2+{:.2}:y=(oh-ih)/2+{:.2}:color=black",
                    orig_w, orig_h, pos_x, pos_y
                ));
            } else {
                v_filter_chain.push(format!("scale=w={}:h={}", ex_clip.width, ex_clip.height));
            }

            filter_complex.push_str(&format!(
                "[{}:v]{}[v_proc_{}];",
                i,
                v_filter_chain.join(","),
                i
            ));

            if ex_clip.has_audio {
                let mut a_filter_chain = Vec::new();
                if let Some(ref trans) = clip.transition {
                    if trans == "fade" && clip.duration > 1.0 {
                        a_filter_chain.push("afade=t=in:st=0:d=0.5".to_string());
                        a_filter_chain
                            .push(format!("afade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
                    }
                }
                if !a_filter_chain.is_empty() {
                    filter_complex.push_str(&format!(
                        "[{}:a]{}[a_proc_{}];",
                        i,
                        a_filter_chain.join(","),
                        i
                    ));
                }
            } else {
                filter_complex.push_str(&format!(
                    "anullsrc=r=44100:cl=stereo,atrim=0:{:.3}[a_proc_{}];",
                    clip.duration, i
                ));
            }
        }

        // 2. Concat processed Video & Audio inputs (Track 0)
        if plan.clips.len() == 1 {
            let ex_clip = &plan.clips[0];
            let clip = &ex_clip.clip;

            let v_label = "[v_proc_0]".to_string();
            let a_label =
                if !ex_clip.has_audio || (clip.transition.is_some() && clip.duration > 1.0) {
                    "[a_proc_0]".to_string()
                } else {
                    "[0:a]".to_string()
                };

            filter_complex.push_str(&format!("{}null[v_concat];", v_label));
            filter_complex.push_str(&format!("{}anull[a_concat];", a_label));
        } else {
            for i in 0..plan.clips.len() {
                let ex_clip = &plan.clips[i];
                let clip = &ex_clip.clip;

                let v_label = format!("[v_proc_{}]", i);
                let a_label =
                    if !ex_clip.has_audio || (clip.transition.is_some() && clip.duration > 1.0) {
                        format!("[a_proc_{}]", i)
                    } else {
                        format!("[{}:a]", i)
                    };

                filter_complex.push_str(&format!("{}{}", v_label, a_label));
            }
            filter_complex.push_str(&format!(
                "concat=n={}:v=1:a=1[v_concat][a_concat];",
                plan.clips.len()
            ));
        }

        // Apply main track volume
        filter_complex.push_str(&format!(
            "[a_concat]volume={:.2}[a_main_vol]",
            plan.track_0_vol
        ));

        // 3. Process Track 1 Overlay inputs
        let n = plan.clips.len();
        let mut current_bg = "[v_concat]".to_string();
        for (j, ex_clip) in plan.overlay_clips.iter().enumerate() {
            let clip = &ex_clip.clip;
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

            let has_geom_transform =
                scale_factor != 1.0 || rot_deg != 0.0 || pos_x != 0.0 || pos_y != 0.0;
            if has_geom_transform {
                let orig_w = ex_clip.width;
                let orig_h = ex_clip.height;

                if scale_factor != 1.0 {
                    v_filter_chain.push(format!(
                        "scale=w=iw*{:.2}:h=ih*{:.2}",
                        scale_factor, scale_factor
                    ));
                }
                if rot_deg != 0.0 {
                    v_filter_chain.push(format!("rotate={:.2}*PI/180:fillcolor=black", rot_deg));
                }
                v_filter_chain.push(format!(
                    "pad=w={}:h={}:x=(ow-iw)/2+{:.2}:y=(oh-ih)/2+{:.2}:color=black@0",
                    orig_w, orig_h, pos_x, pos_y
                ));
            } else {
                v_filter_chain.push(format!("scale=w={}:h={}", ex_clip.width, ex_clip.height));
            }

            let v_label = format!("[v_overlay_proc_{}]", j);
            if !filter_complex.is_empty() && !filter_complex.ends_with(';') {
                filter_complex.push(';');
            }
            filter_complex.push_str(&format!(
                "[{}:v]{}{};",
                idx,
                v_filter_chain.join(","),
                v_label
            ));

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
        let audio_start_idx = n + plan.overlay_clips.len();
        for (j, (ex_clip, track_vol)) in plan.audio_clips.iter().enumerate() {
            let clip = &ex_clip.clip;
            let mut bgm_filter_chain = Vec::new();
            if let Some(ref trans) = clip.transition {
                if trans == "fade" && clip.duration > 1.0 {
                    bgm_filter_chain.push("afade=t=in:st=0:d=0.5".to_string());
                    bgm_filter_chain
                        .push(format!("afade=t=out:st={:.3}:d=0.5", clip.duration - 0.5));
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
        if !plan.audio_clips.is_empty() {
            if !filter_complex.is_empty() && !filter_complex.ends_with(';') {
                filter_complex.push(';');
            }
            filter_complex.push_str("[a_main_vol]");
            for j in 0..plan.audio_clips.len() {
                filter_complex.push_str(&format!("[delayed_bgm_{}]", j));
            }
            filter_complex.push_str(&format!(
                "amix=inputs={}:duration=first[a_mixed]",
                1 + plan.audio_clips.len()
            ));
        }

        if !filter_complex.is_empty() {
            cmd.arg("-filter_complex").arg(filter_complex);
        }
        cmd.arg("-map").arg(&current_bg);

        if !plan.audio_clips.is_empty() {
            cmd.arg("-map").arg("[a_mixed]");
        } else {
            cmd.arg("-map").arg("[a_main_vol]");
        }
        cmd.arg("-pix_fmt").arg("yuv420p");
        cmd.arg(&plan.export_path);

        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::piped());

        (cmd, total_duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_clip(id: &str, duration: f64, start: f64) -> Clip {
        Clip {
            id: id.to_string(),
            name: format!("{}.mp4", id),
            path: format!("/mock/path/{}.mp4", id),
            start,
            duration,
            color: "#ffffff".to_string(),
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
        }
    }

    #[test]
    fn test_single_video_no_audio() {
        let clip = create_mock_clip("clip1", 5.0, 0.0);
        let plan = ExportPlan {
            clips: vec![ExportClip {
                clip,
                has_audio: false,
                width: 1920,
                height: 1080,
                is_image: false,
            }],
            overlay_clips: vec![],
            audio_clips: vec![],
            track_0_vol: 1.0,
            export_path: PathBuf::from("output.mp4"),
        };

        let (cmd, duration) = FfmpegCommandBuilder::build(&plan);
        assert_eq!(duration, 5.0);

        // Check command args
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();

        // Output path should be at the end
        assert_eq!(args.last().unwrap(), "output.mp4");

        // -filter_complex should generate silent audio source
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let fc = &args[fc_idx + 1];
        assert!(fc.contains("anullsrc="));
        assert!(fc.contains("null[v_concat]"));
        assert!(fc.contains("anull[a_concat]"));
    }

    #[test]
    fn test_concat_multiple_videos() {
        let clip1 = create_mock_clip("clip1", 5.0, 0.0);
        let clip2 = create_mock_clip("clip2", 10.0, 5.0);
        let plan = ExportPlan {
            clips: vec![
                ExportClip {
                    clip: clip1,
                    has_audio: true,
                    width: 1920,
                    height: 1080,
                    is_image: false,
                },
                ExportClip {
                    clip: clip2,
                    has_audio: true,
                    width: 1920,
                    height: 1080,
                    is_image: false,
                },
            ],
            overlay_clips: vec![],
            audio_clips: vec![],
            track_0_vol: 0.8,
            export_path: PathBuf::from("output.mp4"),
        };

        let (cmd, duration) = FfmpegCommandBuilder::build(&plan);
        assert_eq!(duration, 15.0);

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let fc = &args[fc_idx + 1];

        // concat node should have n=2
        assert!(fc.contains("concat=n=2:v=1:a=1"));
        // should apply track_0_vol volume=0.8
        assert!(fc.contains("volume=0.80"));
    }

    #[test]
    fn test_overlay_and_bgm() {
        let main_clip = create_mock_clip("main", 20.0, 0.0);
        let overlay = create_mock_clip("overlay", 5.0, 5.0);
        let bgm = create_mock_clip("bgm", 15.0, 2.0);

        let plan = ExportPlan {
            clips: vec![ExportClip {
                clip: main_clip,
                has_audio: true,
                width: 1920,
                height: 1080,
                is_image: false,
            }],
            overlay_clips: vec![ExportClip {
                clip: overlay,
                has_audio: false,
                width: 640,
                height: 360,
                is_image: false,
            }],
            audio_clips: vec![(
                ExportClip {
                    clip: bgm,
                    has_audio: true,
                    width: 0,
                    height: 0,
                    is_image: false,
                },
                0.5,
            )],
            track_0_vol: 1.0,
            export_path: PathBuf::from("output.mp4"),
        };

        let (cmd, duration) = FfmpegCommandBuilder::build(&plan);
        assert_eq!(duration, 20.0);

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let fc = &args[fc_idx + 1];

        // overlay filter syntax check
        assert!(fc.contains("overlay="));
        assert!(fc.contains("enable='between(t,5.000,10.000)'"));

        // bgm delay check (2.0s delay = 2000ms)
        assert!(fc.contains("adelay=2000|2000"));
        // bgm mixing check
        assert!(fc.contains("amix=inputs=2"));
    }
}
