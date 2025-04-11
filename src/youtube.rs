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
    println!("Downloading YouTube video from: {}", url);
    println!("Output directory: {}", output_dir.display());
    
    // Use a more specific output pattern with a timestamp to avoid conflicts
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let output_pattern = format!("yt_download_{}_%(title)s.%(ext)s", timestamp);
    let output_path = output_dir.join(&output_pattern);
    
    println!("Using output pattern: {}", output_pattern);
    
    let output = Command::new("yt-dlp")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("--no-playlist")  // Avoid downloading playlists
        .arg(url)
        .output()
        .context("Failed to download YouTube video")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        println!("yt-dlp error output: {}", error_msg);
        anyhow::bail!("yt-dlp error: {}", error_msg);
    }

    println!("Download completed successfully");
    
    // Find the most recently modified FILE (not directory) in the output directory
    let video_path = std::fs::read_dir(output_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            // Only include files, not directories
            match entry.file_type() {
                Ok(file_type) => file_type.is_file(),
                Err(_) => false,
            }
        })
        .filter(|entry| {
            // Only include files that match our timestamp pattern
            entry.file_name().to_string_lossy().starts_with(&format!("yt_download_{}", timestamp))
        })
        .max_by_key(|entry| entry.metadata().map(|m| m.modified().unwrap()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
        .map(|entry| entry.path())
        .context("No video file found in output directory after download")?;

    println!("Found downloaded video file: {}", video_path.display());
    
    // Verify that the file exists and is not a directory
    if !video_path.exists() || video_path.is_dir() {
        anyhow::bail!("Invalid video file path: {} (exists: {}, is_dir: {})", 
            video_path.display(), 
            video_path.exists(), 
            video_path.is_dir()
        );
    }

    Ok(video_path)
}

// Convert video to audio
pub fn convert_to_audio(video_path: &PathBuf) -> Result<PathBuf> {
    println!("Converting video to audio: {}", video_path.display());
    
    // Validate input file
    if !video_path.exists() {
        anyhow::bail!("Video file does not exist: {}", video_path.display());
    }
    
    if video_path.is_dir() {
        anyhow::bail!("Expected a file but got a directory: {}", video_path.display());
    }
    
    let audio_path = video_path.with_extension("mp3");
    println!("Output audio path: {}", audio_path.display());
    
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-vn")  // No video
        .arg("-acodec")
        .arg("libmp3lame")  // Use MP3 codec
        .arg("-q:a")
        .arg("4")  // Quality setting
        .arg(&audio_path)
        .output()
        .context("Failed to execute ffmpeg command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        println!("ffmpeg error output: {}", error_msg);
        anyhow::bail!("ffmpeg error: {}", error_msg);
    }

    println!("Successfully converted video to audio");
    
    // Verify the output file exists
    if !audio_path.exists() {
        anyhow::bail!("Audio conversion failed: output file does not exist");
    }

    Ok(audio_path)
}
