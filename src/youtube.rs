use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

// Check if yt-dlp is installed
pub fn check_yt_dlp_installed() -> Result<()> {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .context("yt-dlp is not installed or not found in PATH")?;
    Ok(())
}

// Check if ffmpeg is installed
pub fn check_ffmpeg_installed() -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("ffmpeg is not installed or not found in PATH")?;
    Ok(())
}

// Download YouTube video
pub fn download_youtube_video(url: &str, output_dir: &PathBuf) -> Result<PathBuf> {
    let output = Command::new("yt-dlp")
        .arg("-o")
        .arg(output_dir.join("%(title)s.%(ext)s").to_str().unwrap())
        .arg(url)
        .output()
        .context("Failed to download YouTube video")?;

    if !output.status.success() {
        anyhow::bail!("yt-dlp error: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Assuming the video is downloaded as the first file in the output directory
    let video_path = std::fs::read_dir(output_dir)?
        .next()
        .context("No video file found in output directory")?
        .map(|entry| entry.path())?;

    Ok(video_path)
}

// Convert video to audio
pub fn convert_to_audio(video_path: &PathBuf) -> Result<PathBuf> {
    let audio_path = video_path.with_extension("mp3");
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg(&audio_path)
        .output()
        .context("Failed to convert video to audio")?;

    if !output.status.success() {
        anyhow::bail!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(audio_path)
}
