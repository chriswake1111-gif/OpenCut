use std::process::Command;

#[allow(dead_code)]
pub fn check_ffmpeg_available() -> bool {
    #[cfg(target_os = "windows")]
    let cmd = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    let cmd = "ffmpeg";

    Command::new(cmd).arg("-version").output().is_ok()
}

#[allow(dead_code)]
pub fn check_ffprobe_available() -> bool {
    #[cfg(target_os = "windows")]
    let cmd = "ffprobe.exe";
    #[cfg(not(target_os = "windows"))]
    let cmd = "ffprobe";

    Command::new(cmd).arg("-version").output().is_ok()
}
