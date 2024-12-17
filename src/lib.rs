mod client;
mod models;
mod utils;
mod config;

// Re-export types needed for the public API
pub use client::{check_service, get_job_status, list_jobs, transcribe_file};
pub use models::{FileInfo, JobResponse, Segment, TranscriptionResponse};
pub use utils::{collect_audio_files, is_supported_audio_format, save_markdown_response};
pub use config::Config;

// Re-export command line types
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Whisper transcription client", long_about = None)]
pub struct Args {
    /// Command to execute (transcribe, list-jobs, status)
    #[arg(value_enum)]
    pub command: Command,

    /// Path to audio file or directory of audio files (required for transcribe command)
    #[arg(name = "PATH", required_if_eq("command", "transcribe"))]
    pub path: Option<std::path::PathBuf>,

    /// Process directory recursively (only valid with directory input)
    #[arg(short, long)]
    pub recursive: bool,

    /// Job ID (required for status command)
    #[arg(name = "JOB_ID", long, required_if_eq("command", "status"))]
    pub job_id: Option<String>,

    /// Show detailed output including segments
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Command {
    /// Transcribe an audio file or directory
    Transcribe,
    /// List all jobs
    ListJobs,
    /// Get status of a specific job
    Status,
}