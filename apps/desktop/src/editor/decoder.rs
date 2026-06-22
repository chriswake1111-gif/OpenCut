use std::process::Command;
use std::sync::Arc;

/// Probe a video file using ffprobe to extract (width, height, duration).
pub fn probe_video_metadata(path: &str) -> Option<(u32, u32, f64)> {
    println!("Probing video metadata for: {}", path);

    // 1. Try to get width, height, and stream duration
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,duration",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        println!("ffprobe failed to run on stream query");
        return None;
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout_str.trim().split(',').collect();
    if parts.len() < 2 {
        println!("ffprobe returned invalid format: {}", stdout_str);
        return None;
    }

    let width: u32 = parts[0].trim().parse().ok()?;
    let height: u32 = parts[1].trim().parse().ok()?;

    let mut duration: f64 = parts
        .get(2)
        .and_then(|&s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);

    // 2. If stream duration is missing (0.0 or parse error), query format duration
    if duration == 0.0 {
        let format_output = Command::new("ffprobe")
            .args(&[
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
                path,
            ])
            .output()
            .ok()?;

        if format_output.status.success() {
            let format_stdout = String::from_utf8_lossy(&format_output.stdout);
            if let Ok(d) = format_stdout.trim().parse::<f64>() {
                duration = d;
            }
        }
    }

    println!(
        "Probe success: {}x{}, duration: {}s",
        width, height, duration
    );
    Some((width, height, duration))
}

/// Extract a single frame at the given timestamp scaled to out_width x out_height.
/// Returns a Vec<u8> containing raw BGRA bytes.
pub fn extract_video_frame(
    path: &str,
    time: f64,
    out_width: u32,
    out_height: u32,
) -> Option<Vec<u8>> {
    // Format timestamp with 3 decimal places
    let time_str = format!("{:.3}", time);
    let scale_str = format!("scale={}:{}", out_width, out_height);

    let output = Command::new("ffmpeg")
        .args(&[
            "-ss",
            &time_str,
            "-i",
            path,
            "-vf",
            &scale_str,
            "-f",
            "image2pipe",
            "-pix_fmt",
            "bgra",
            "-vcodec",
            "rawvideo",
            "-vframes",
            "1",
            "-",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        // If it failed, check stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("ffmpeg frame extraction failed: {}", stderr);
        return None;
    }

    let expected_bytes = (out_width * out_height * 4) as usize;
    if output.stdout.len() < expected_bytes {
        println!(
            "ffmpeg returned less bytes than expected: {} < {}",
            output.stdout.len(),
            expected_bytes
        );
        return None;
    }

    // Return the first expected_bytes in case ffmpeg outputted trailing data
    Some(output.stdout[0..expected_bytes].to_vec())
}

/// Check if a media file has an audio stream using ffprobe.
pub fn has_audio_stream(path: &str) -> bool {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            path,
        ])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            return !stdout_str.trim().is_empty();
        }
    }
    false
}

/// Extract low-resolution audio waveform amplitude points from a media file.
/// Sampling at 20 Hz to get a compact representation of the volume envelope.
pub fn extract_audio_waveform(path: &str) -> Option<Vec<f32>> {
    println!("Extracting audio waveform for: {}", path);
    let output = Command::new("ffmpeg")
        .args(&[
            "-i",
            path,
            "-ac",
            "1",
            "-filter:a",
            "aresample=20",
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        println!("ffmpeg waveform extraction failed on: {}", path);
        return None;
    }

    let bytes = output.stdout;
    if bytes.len() % 4 != 0 {
        return None;
    }

    let count = bytes.len() / 4;
    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * 4;
        let buf = [
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
        ];
        let val = f32::from_le_bytes(buf).abs();
        samples.push(val);
    }

    // Normalize: find the max value and scale all values to [0.0, 1.0]
    let mut max_val = 0.0f32;
    for &val in &samples {
        if val > max_val {
            max_val = val;
        }
    }

    if max_val > 0.0 {
        for val in &mut samples {
            *val /= max_val;
        }
    }

    println!(
        "Audio waveform extraction success: {} samples",
        samples.len()
    );
    Some(samples)
}

/// Extract full interleaved stereo f32 PCM audio samples at 44100 Hz.
pub fn extract_audio_samples(path: &str) -> Option<Vec<f32>> {
    println!("Extracting full audio samples for: {}", path);

    // -ac 2 (stereo), -ar 44100 (44.1kHz), -f f32le (f32 little endian)
    let output = Command::new("ffmpeg")
        .args(&["-i", path, "-ac", "2", "-ar", "44100", "-f", "f32le", "-"])
        .output()
        .ok()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("ffmpeg audio extraction failed: {}", stderr);
        return None;
    }

    let bytes = output.stdout;
    if bytes.len() % 4 != 0 {
        println!("Audio output length not a multiple of 4: {}", bytes.len());
        return None;
    }

    let count = bytes.len() / 4;
    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * 4;
        let buf = [
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
        ];
        samples.push(f32::from_le_bytes(buf));
    }

    println!(
        "Audio sample extraction success: {} samples (stereo interleaved)",
        samples.len()
    );
    Some(samples)
}

/// Load a static image file, decode and resize it to out_width x out_height.
/// Returns a Vec<u8> containing raw BGRA bytes.
pub fn extract_static_image(path: &str, out_width: u32, out_height: u32) -> Option<Vec<u8>> {
    println!("Loading static image: {}", path);
    let img = image::open(path).ok()?;
    let img_rgba = img.to_rgba8();

    let resized = image::imageops::resize(
        &img_rgba,
        out_width,
        out_height,
        image::imageops::FilterType::Triangle,
    );

    let mut bgra_bytes = Vec::with_capacity((out_width * out_height * 4) as usize);
    for pixel in resized.pixels() {
        bgra_bytes.push(pixel[2]); // B
        bgra_bytes.push(pixel[1]); // G
        bgra_bytes.push(pixel[0]); // R
        bgra_bytes.push(pixel[3]); // A
    }

    Some(bgra_bytes)
}

pub struct FrameCache {
    inner: std::sync::Mutex<FrameCacheInner>,
}

struct FrameCacheInner {
    map: std::collections::HashMap<(String, u32), Arc<image::RgbaImage>>,
    order: std::collections::VecDeque<(String, u32)>,
    max_capacity: usize,
}

impl FrameCache {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            inner: std::sync::Mutex::new(FrameCacheInner {
                map: std::collections::HashMap::new(),
                order: std::collections::VecDeque::new(),
                max_capacity,
            }),
        }
    }

    pub fn get(&self, path: &str, time: f64) -> Option<Arc<image::RgbaImage>> {
        let key = (path.to_string(), (time / 0.016).round() as u32);
        let mut inner = self.inner.lock().unwrap();
        if let Some(img) = inner.map.get(&key).cloned() {
            // Move to the back of order (most recently used)
            if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                inner.order.remove(pos);
            }
            inner.order.push_back(key);
            Some(img)
        } else {
            None
        }
    }

    pub fn insert(&self, path: &str, time: f64, img: Arc<image::RgbaImage>) {
        let key = (path.to_string(), (time / 0.016).round() as u32);
        let mut inner = self.inner.lock().unwrap();

        // Remove if existing to refresh position
        if inner.map.contains_key(&key) {
            if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                inner.order.remove(pos);
            }
        } else if inner.order.len() >= inner.max_capacity {
            if let Some(oldest_key) = inner.order.pop_front() {
                inner.map.remove(&oldest_key);
            }
        }

        inner.order.push_back(key.clone());
        inner.map.insert(key, img);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_waveform_extraction_invalid() {
        let result = extract_audio_waveform("non_existent_file.mp3");
        assert!(result.is_none());
    }

    #[test]
    fn test_frame_cache_lru() {
        let cache = FrameCache::new(2);

        let img1 = Arc::new(image::ImageBuffer::new(10, 10));
        let img2 = Arc::new(image::ImageBuffer::new(10, 10));
        let img3 = Arc::new(image::ImageBuffer::new(10, 10));

        // Insert first two
        cache.insert("path1", 0.0, img1.clone());
        cache.insert("path1", 0.033, img2.clone());

        // Both should be in cache
        assert!(cache.get("path1", 0.0).is_some());
        assert!(cache.get("path1", 0.033).is_some());

        // Insert third one (this should evict the oldest: path1 @ 0.0, since we read path1 @ 0.033 last)
        cache.insert("path1", 0.066, img3.clone());

        // path1 @ 0.0 should be evicted, path1 @ 0.033 and path1 @ 0.066 should remain
        assert!(cache.get("path1", 0.0).is_none());
        assert!(cache.get("path1", 0.033).is_some());
        assert!(cache.get("path1", 0.066).is_some());
    }
}
