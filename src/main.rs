use anyhow::Result;
use colored::*;
use whisper_client::{
    Args, Command,
    check_service, list_jobs, get_job_status, transcribe_file, terminate_job,
    collect_audio_files, save_markdown_response, Config,
};
use clap::Parser;
use std::collections::HashMap;

async fn display_service_info() -> Result<()> {
    // Check service status
    let service_status = match check_service().await {
        Ok(_) => ("✓".green(), "Running"),
        Err(_) => ("✗".red(), "Not available"),
    };
    
    let config = Config::load()?;
    println!("\n{} Service Status: {} {}", "🔍".blue(), service_status.0, service_status.1);
    println!("   URL: {}", config.service_url);
    
    // Only try to get jobs if service is running
    if service_status.1 == "Running" {
        match list_jobs().await {
            Ok(jobs) => {
                // Count jobs by status
                let mut status_counts: HashMap<String, usize> = HashMap::new();
                for job in &jobs {
                    *status_counts.entry(job.status.clone()).or_insert(0) += 1;
                }
                
                // Display job summary
                if !jobs.is_empty() {
                    println!("\n{} Job Summary:", "📊".blue());
                    for (status, count) in status_counts {
                        let status_icon = match status.as_str() {
                            "completed" => "✓".green(),
                            "failed" => "✗".red(),
                            "processing" => "⚙️".blue(),
                            "queued" => "⏳".yellow(),
                            _ => "•".normal(),
                        };
                        println!("   {} {} jobs {}", status_icon, count, status);
                    }
                    
                    // Show most recent active jobs (up to 5)
                    let active_jobs: Vec<_> = jobs.iter()
                        .filter(|j| j.status == "processing" || j.status == "queued")
                        .take(5)
                        .collect();
                    
                    if !active_jobs.is_empty() {
                        println!("\n{} Recent Active Jobs:", "🔄".blue());
                        for job in active_jobs {
                            let status_icon = if job.status == "processing" { "⚙️".blue() } else { "⏳".yellow() };
                            println!(
                                "   {} {} ({}) {}",
                                status_icon,
                                job.job_id,
                                job.status,
                                job.filename.clone().unwrap_or_default()
                            );
                        }
                    }
                } else {
                    println!("\n{} No jobs found", "📊".blue());
                }
            }
            Err(e) => {
                println!("\n{} Could not retrieve jobs: {}", "⚠️".yellow(), e);
            }
        }
    }
    
    // Display available commands
    println!("\n{} Available Commands:", "📋".blue());
    println!("   {} {:<12} - Convert audio file(s) to text", "🎵".green(), "transcribe");
    println!("   {} {:<12} - View all transcription jobs", "📜".green(), "list-jobs");
    println!("   {} {:<12} - Check status of a specific job", "🔍".green(), "status");
    println!("   {} {:<12} - Cancel a running job", "🛑".green(), "terminate");
    
    println!("\n{} Example Usage:", "💡".yellow());
    println!("   whisper-client transcribe audio.mp3");
    println!("   whisper-client transcribe ./recordings/ --recursive");
    println!("   whisper-client list-jobs");
    println!("   whisper-client status --job-id <ID>");
    println!("   whisper-client terminate --job-id <ID>");
    
    println!("\n{} For detailed help on any command:", "ℹ️".blue());
    println!("   whisper-client <command> --help");
    
    Ok(())
}

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

    // Check if service is running for commands that need it
    let needs_service_check = match args.command {
        Some(Command::Info) => false, // Info command handles service check internally
        None => false, // Default to Info command
        _ => true, // All other commands need service check
    };

    if needs_service_check {
        if let Err(e) = check_service().await {
            println!("{} Error: {}", "✗".red(), e);
            println!(
                "{} Start the service with: {}",
                "↳".blue(),
                "docker compose up -d".bold()
            );
            std::process::exit(1);
        }
    }

    match args.command.unwrap_or(Command::Info) {
        Command::Info => {
            display_service_info().await?;
        },
        Command::Transcribe => {
            // Validate required arguments
            if args.path.is_none() {
                println!("{} Error: Missing required PATH argument for transcribe command", "✗".red());
                println!("{} Usage: whisper-client transcribe <PATH>", "ℹ️".blue());
                std::process::exit(1);
            }
            
            let path = args.path.unwrap();
            
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
            // Validate required arguments
            if args.job_id.is_none() {
                println!("{} Error: Missing required --job-id argument for status command", "✗".red());
                println!("{} Usage: whisper-client status --job-id <JOB_ID>", "ℹ️".blue());
                std::process::exit(1);
            }
            
            let job_id = args.job_id.unwrap();
            match get_job_status(&job_id, true).await {
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
        Command::Terminate => {
            // Validate required arguments
            if args.job_id.is_none() {
                println!("{} Error: Missing required --job-id argument for terminate command", "✗".red());
                println!("{} Usage: whisper-client terminate --job-id <JOB_ID>", "ℹ️".blue());
                std::process::exit(1);
            }
            
            let job_id = args.job_id.unwrap();
            println!("\n{} Attempting to terminate job {}...", "→".blue(), job_id);
            
            match terminate_job(&job_id).await {
                Ok(job) => {
                    println!("{} Job terminated successfully", "✓".green());
                    println!("Status: {}", job.status);
                    if !job.message.is_empty() {
                        println!("Message: {}", job.message);
                    }
                }
                Err(e) => {
                    println!("{} Error terminating job: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
