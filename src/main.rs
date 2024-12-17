use anyhow::Result;
use colored::*;
use std::io::Write;
use whisper_client::{
    Args, Command,
    check_service, list_jobs, get_job_status, transcribe_file,
    collect_audio_files, save_markdown_response,
};
use clap::Parser;

async fn process_batch(files: Vec<std::path::PathBuf>, verbose: bool) -> Result<()> {
    let total = files.len();
    println!("\n{} Found {} files to process", "→".blue(), total);
    
    for (index, file) in files.into_iter().enumerate() {
        println!("\n{} Processing file {} of {}: {}", "→".blue(), index + 1, total, file.display());
        println!("{} Sending file to Whisper service...", "→".blue());
        
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
