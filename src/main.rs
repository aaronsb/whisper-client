use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use std::io::Write;

const SERVICE_URL: &str = "http://localhost:8000";

#[derive(Parser, Debug)]
#[command(author, version, about = "Whisper transcription client", long_about = None)]
struct Args {
    /// Command to execute (transcribe, list-jobs, status)
    #[arg(value_enum)]
    command: Command,

    /// Path to audio file or directory of audio files (required for transcribe command)
    #[arg(name = "PATH", required_if_eq("command", "transcribe"))]
    path: Option<PathBuf>,

    /// Process directory recursively (only valid with directory input)
    #[arg(short, long)]
    recursive: bool,

    /// Job ID (required for status command)
    #[arg(name = "JOB_ID", long, required_if_eq("command", "status"))]
    job_id: Option<String>,

    /// Show detailed output including segments
    #[arg(short, long)]
    verbose: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Command {
    /// Transcribe an audio file or directory
    Transcribe,
    /// List all jobs
    ListJobs,
    /// Get status of a specific job
    Status,
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

fn collect_audio_files(path: &PathBuf, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if path.is_file() {
        if is_supported_audio_format(path) {
            files.push(path.clone());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && is_supported_audio_format(&path) {
                files.push(path);
            } else if recursive && path.is_dir() {
                files.extend(collect_audio_files(&path, true)?);
            }
        }
    }
    
    Ok(files)
}

async fn check_job_status(job_id: &str) -> Result<JobResponse> {
    get_job_status(job_id).await
}

async fn list_jobs() -> Result<Vec<JobResponse>> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/jobs", SERVICE_URL))
        .send()
        .await
        .context("Failed to list jobs")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Service error: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let jobs: Vec<JobResponse> = response
        .json()
        .await
        .context("Failed to parse jobs response")?;

    Ok(jobs)
}

async fn terminate_job(job_id: &str) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let response = client
        .delete(&format!("{}/jobs/{}", SERVICE_URL, job_id))
        .send()
        .await
        .context("Failed to terminate job")?;

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

    Ok(job_status)
}

async fn get_job_status(job_id: &str) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/status/{}", SERVICE_URL, job_id))
        .send()
        .await
        .context("Failed to get job status")?;

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

    Ok(job_status)
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

    println!("{} Job started with ID: {}", "→".blue(), job_response.job_id);

    // Set up polling with Ctrl+C handling
    let job_id = job_response.job_id.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n{} Received interrupt, terminating job...", "⚠".yellow());
                if let Ok(terminated) = terminate_job(&job_id).await {
                    println!("{} Job terminated: {}", "✓".green(), terminated.message);
                }
                anyhow::bail!("Job terminated by user");
            }
            _ = interval.tick() => {
                let status = check_job_status(&job_id).await?;
                print!("\r{} {}", "⋯".blue(), status.message);
                std::io::stdout().flush()?;

                match status.status.as_str() {
                    "completed" => {
                        if let Some(ref result) = status.result {
                            println!("\n{} Transcription processing complete!", "✓".green());
                            return Ok((result.clone(), status));
                        }
                        anyhow::bail!("No result in completed job");
                    }
                    "failed" => {
                        anyhow::bail!("\nTranscription failed: {}", status.message);
                    }
                    _ => {}
                }
            }
        }
    }
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

async fn process_batch(files: Vec<PathBuf>, verbose: bool) -> Result<()> {
    let total = files.len();
    println!("\n{} Found {} files to process", "→".blue(), total);
    
    for (index, file) in files.into_iter().enumerate() {
        println!("\n{} Processing file {} of {}: {}", "→".blue(), index + 1, total, file.display());
        
        match transcribe_file(&file).await {
            Ok((transcription, job_info)) => {
                let output_path = save_markdown_response(&transcription, &file, &job_info)?;
                println!("{} Saved transcript to: {}", "✓".green(), output_path.display());

                if verbose {
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
            }
            Err(e) => {
                println!("\n{} Error processing {}: {}", "✗".red(), file.display(), e);
                // Continue with next file instead of exiting
                continue;
            }
        }
    }
    
    println!("\n{} Batch processing complete!", "✓".green());
    Ok(())
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

    match args.command {
        Command::Transcribe => {
            let path = args.path.unwrap(); // Safe due to required_if_eq
            
            // Collect files to process
            let files = collect_audio_files(&path, args.recursive)?;
            
            if files.is_empty() {
                println!("{} No compatible audio files found", "✗".red());
                std::process::exit(1);
            }
            
            process_batch(files, args.verbose).await?;
        }
        Command::ListJobs => {
            match list_jobs().await {
                Ok(jobs) => {
                    println!("\n{}", "Jobs:".bold());
                    for job in jobs {
                        let status_color = match job.status.as_str() {
                            "completed" => "✓".green(),
                            "failed" => "✗".red(),
                            _ => "⋯".blue(),
                        };
                        
                        println!(
                            "{} {} - {} {}",
                            status_color,
                            job.job_id,
                            job.status,
                            job.filename.unwrap_or_default()
                        );
                        
                        if args.verbose {
                            if let Some(created_at) = job.created_at {
                                let datetime = chrono::DateTime::from_timestamp(created_at as i64, 0)
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_else(|| "Unknown".to_string());
                                println!("   Created: {}", datetime);
                            }
                            if !job.message.is_empty() {
                                println!("   Message: {}", job.message);
                            }
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("\n{} Error: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
        }
        Command::Status => {
            let job_id = args.job_id.unwrap(); // Safe due to required_if_eq
            match get_job_status(&job_id).await {
                Ok(job) => {
                    let status_color = match job.status.as_str() {
                        "completed" => "✓".green(),
                        "failed" => "✗".red(),
                        _ => "⋯".blue(),
                    };
                    
                    println!("\n{} Status for job {}:", status_color, job.job_id);
                    println!("Status: {}", job.status);
                    if let Some(filename) = job.filename {
                        println!("File: {}", filename);
                    }
                    if let Some(created_at) = job.created_at {
                        let datetime = chrono::DateTime::from_timestamp(created_at as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        println!("Created: {}", datetime);
                    }
                    if !job.message.is_empty() {
                        println!("Message: {}", job.message);
                    }
                    
                    if args.verbose && job.status == "completed" {
                        if let Some(result) = job.result {
                            println!("\n{}", "Transcription:".bold());
                            println!("{}\n", result.text);

                            println!("{}", "Segments:".bold());
                            for segment in result.segments {
                                println!(
                                    "{}s -> {}s: {}",
                                    segment.start, segment.end, segment.text
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("\n{} Error: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
