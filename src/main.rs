use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

const SERVICE_URL: &str = "http://localhost:8000";

#[derive(Parser, Debug)]
#[command(author, version, about = "Whisper transcription client", long_about = None)]
struct Args {
    /// Path to the audio file
    #[arg(name = "FILE")]
    file: PathBuf,

    /// Show detailed output including segments
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Segment {
    id: i32,
    seek: i32,
    start: f64,
    end: f64,
    text: String,
    tokens: Vec<i64>,
    temperature: f64,
    avg_logprob: f64,
    compression_ratio: f64,
    no_speech_prob: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TranscriptionResponse {
    text: String,
    segments: Vec<Segment>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FileInfo {
    name: String,
    size: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct JobResponse {
    job_id: String,
    status: String,
    #[serde(default)]
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<TranscriptionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_info: Option<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

async fn check_service() -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health", SERVICE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .context("Failed to connect to Whisper service")?;

    if !response.status().is_success() {
        anyhow::bail!("Service health check failed");
    }

    Ok(())
}

fn is_supported_audio_format(path: &PathBuf) -> bool {
    let supported = ["mp3", "wav", "m4a", "ogg", "flac"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

async fn check_job_status(job_id: &str) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let mut attempts = 0;
    let max_attempts = 60; // 5 minutes with 5 second intervals

    while attempts < max_attempts {
        let response = client
            .get(&format!("{}/status/{}", SERVICE_URL, job_id))
            .send()
            .await
            .context("Failed to check job status")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Service error: {}",
                response.text().await.unwrap_or_default()
            );
        }

        let job_status: JobResponse = response
            .json()
            .await
            .context("Failed to parse job status response")?;

        match job_status.status.as_str() {
            "completed" => {
                return Ok(job_status);
            }
            "failed" => {
                anyhow::bail!("Transcription failed: {}", job_status.message);
            }
            _ => {
                print!("\r{} {}", "⋯".blue(), job_status.message);
                std::io::Write::flush(&mut std::io::stdout())?;
                sleep(Duration::from_secs(5)).await;
                attempts += 1;
            }
        }
    }

    anyhow::bail!("Timeout waiting for transcription to complete")
}

async fn transcribe_file(path: &PathBuf) -> Result<(TranscriptionResponse, JobResponse)> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }

    if !is_supported_audio_format(path) {
        anyhow::bail!(
            "Unsupported audio format. Supported formats: mp3, wav, m4a, ogg, flac"
        );
    }

    let file_name = path
        .file_name()
        .context("Invalid file name")?
        .to_str()
        .context("Invalid file name encoding")?;

    let file_content = tokio::fs::read(path)
        .await
        .context("Failed to read audio file")?;

    let mime_type = mime_guess::from_path(path)
        .first()
        .context("Could not determine MIME type")?;

    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(file_content)
            .file_name(file_name.to_string())
            .mime_str(mime_type.as_ref())
            .context("Invalid MIME type")?,
    );

    println!("{} Sending file to Whisper service...", "→".blue());

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/transcribe/", SERVICE_URL))
        .multipart(form)
        .timeout(Duration::from_secs(3600))
        .send()
        .await
        .context("Failed to send file to service")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Service error: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let job_response: JobResponse = response
        .json()
        .await
        .context("Failed to parse service response")?;

    // Poll for job completion
    let mut job_status = check_job_status(&job_response.job_id).await?;
    let transcription = job_status.result.take()
        .ok_or_else(|| anyhow::anyhow!("No result in completed job"))?;

    println!("\r{} Transcription processing complete!", "✓".green());

    Ok((transcription, job_status))
}

fn save_markdown_response(
    response: &TranscriptionResponse,
    input_path: &PathBuf,
    job_info: &JobResponse,
) -> Result<PathBuf> {
    let parent = input_path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = input_path
        .file_stem()
        .context("Invalid file name")?
        .to_str()
        .context("Invalid file name encoding")?;
    
    let output_path = parent.join(format!("{}.md", stem));
    
    // Calculate total duration from last segment
    let duration = response.segments.last()
        .map(|seg| seg.end)
        .unwrap_or(0.0);
    
    // Format duration as minutes:seconds
    let minutes = (duration / 60.0).floor();
    let seconds = (duration % 60.0).round();
    
    // Build markdown content
    let mut markdown = String::new();
    
    // Add transcription text
    markdown.push_str(&response.text);
    markdown.push_str("\n\n---\n\n");
    
    // Add file information section
    markdown.push_str("## Audio File Information\n\n");
    markdown.push_str(&format!("- **Source File:** {}\n", input_path.file_name().unwrap().to_string_lossy()));
    if let Some(file_info) = &job_info.file_info {
        markdown.push_str(&format!("- **File Size:** {} bytes\n", file_info.size));
    }
    markdown.push_str(&format!("- **Duration:** {}:{:02}\n", minutes, seconds));
    if let Some(created_at) = job_info.created_at {
        let datetime = chrono::DateTime::from_timestamp(created_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        markdown.push_str(&format!("- **Transcribed:** {}\n", datetime));
    }
    
    std::fs::write(&output_path, markdown)?;
    
    Ok(output_path)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("\n{} {}", "🎤".blue(), "Whisper Transcription".bold());

    // Check if service is running
    if let Err(e) = check_service().await {
        println!("{} Error: {}", "✗".red(), e);
        println!(
            "{} Start the service with: {}",
            "↳".blue(),
            "docker compose up -d".bold()
        );
        std::process::exit(1);
    }

    // Transcribe file
    match transcribe_file(&args.file).await {
        Ok((transcription, job_info)) => {
            // Save markdown response
            let output_path = save_markdown_response(&transcription, &args.file, &job_info)?;
            println!("{} Saved transcript to: {}", "✓".green(), output_path.display());

            if args.verbose {
                println!("\n{}", "Transcription:".bold());
                println!("{}\n", transcription.text);

                println!("{}", "Segments:".bold());
                for segment in transcription.segments {
                    println!(
                        "{}s -> {}s: {}",
                        segment.start, segment.end, segment.text
                    );
                }
                println!();
            }

            println!("{} Transcription complete!", "✓".green());
            Ok(())
        }
        Err(e) => {
            println!("\n{} Error: {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
}
