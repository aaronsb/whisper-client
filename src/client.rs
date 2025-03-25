use anyhow::{Context, Result};
use reqwest::multipart;
use std::path::PathBuf;
use std::time::Duration;
use crate::models::{JobResponse, TranscriptionResponse};
use crate::config::Config;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

lazy_static::lazy_static! {
    static ref CONFIG: Config = Config::load().expect("Failed to load config");
}

pub async fn check_service() -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health", CONFIG.service_url))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .context("Failed to connect to Whisper service")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Service health check failed with status {}: {}", status, error_text);
    }

    Ok(())
}

pub async fn get_job_status(job_id: &str, include_transcript: bool) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/status/{}?include_transcript={}", 
            CONFIG.service_url, 
            job_id,
            include_transcript))
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

pub async fn list_jobs() -> Result<Vec<JobResponse>> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/jobs", CONFIG.service_url))
        .send()
        .await
        .context("Failed to list jobs")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Service error: {}",
            response.text().await.unwrap_or_default()
        );
    }

    // Get the response text to handle malformed JSON
    let text = response.text().await.context("Failed to get response text")?;
    
    // Fix malformed JSON if needed
    let fixed_text = if !text.trim().starts_with('{') {
        format!("{{{}", text)
    } else {
        text
    };
    
    // Try to parse the fixed JSON
    match serde_json::from_str::<serde_json::Value>(&fixed_text) {
        Ok(value) => {
            if let Some(jobs_array) = value.get("jobs").and_then(|j| j.as_array()) {
                let jobs: Vec<JobResponse> = serde_json::from_value(jobs_array.clone().into())
                    .context("Failed to parse jobs array")?;
                Ok(jobs)
            } else {
                // If there's no "jobs" field, try to parse as a direct array
                let jobs: Vec<JobResponse> = serde_json::from_str(&fixed_text)
                    .context("Failed to parse as direct jobs array")?;
                Ok(jobs)
            }
        },
        Err(_) => {
            // Return empty list as fallback
            Ok(Vec::new())
        }
    }
}

pub async fn terminate_job(job_id: &str) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let response = client
        .delete(&format!("{}/jobs/{}", CONFIG.service_url, job_id))
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

// Helper function to check if a job exists on the server
async fn check_job_exists(job_id: &str) -> Result<bool> {
    match get_job_status(job_id, false).await {
        Ok(_) => Ok(true),
        Err(e) => {
            // Check if the error is due to job not found (404)
            if e.to_string().contains("404") || e.to_string().contains("not found") {
                Ok(false)
            } else {
                // For other errors, propagate them
                Err(e)
            }
        }
    }
}

pub async fn transcribe_file(path: &PathBuf) -> Result<(TranscriptionResponse, JobResponse)> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
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

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/transcribe/", CONFIG.service_url))
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

    // Set up polling
    let job_id = job_response.job_id.clone();
    let mut status_interval = tokio::time::interval(Duration::from_secs(5));
    let mut existence_check_interval = tokio::time::interval(Duration::from_secs(15));
    
    // Create a progress bar
    let progress_bar = ProgressBar::new(100);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    progress_bar.set_position(0);
    
    // Track the last reported progress to avoid duplicate updates
    let mut last_progress_percent = 0.0;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                progress_bar.abandon_with_message("Job terminated by user".red().to_string());
                if let Ok(terminated) = terminate_job(&job_id).await {
                    anyhow::bail!("Job terminated: {}", terminated.message);
                }
                anyhow::bail!("Job terminated by user");
            }
            _ = existence_check_interval.tick() => {
                // Periodically check if the job still exists on the server
                match check_job_exists(&job_id).await {
                    Ok(exists) => {
                        if !exists {
                            progress_bar.abandon_with_message("Job no longer exists on server".red().to_string());
                            anyhow::bail!("Job no longer exists on server. It may have been terminated externally.");
                        }
                    },
                    Err(e) => {
                        eprintln!("Warning: Failed to check if job exists: {}", e);
                        // Continue processing even if check fails
                    }
                }
            }
            _ = status_interval.tick() => {
                match get_job_status(&job_id, false).await {
                    Ok(status) => {
                        match status.status.as_str() {
                            "completed" => {
                                // Complete the progress bar
                                progress_bar.set_position(100);
                                progress_bar.finish_with_message("Transcription completed!".green().to_string());
                                
                                if let Some(ref result) = status.result {
                                    return Ok((result.clone(), status));
                                }
                                anyhow::bail!("No result in completed job");
                            }
                            "failed" => {
                                progress_bar.abandon_with_message(format!("Failed: {}", status.message).red().to_string());
                                anyhow::bail!("Transcription failed: {}", status.message);
                            }
                            "terminated" => {
                                progress_bar.abandon_with_message(format!("Terminated: {}", status.message).red().to_string());
                                anyhow::bail!("Job was terminated: {}", status.message);
                            }
                            "cancelled" => {
                                progress_bar.abandon_with_message(format!("Cancelled: {}", status.message).red().to_string());
                                anyhow::bail!("Job was cancelled: {}", status.message);
                            }
                            "processing" => {
                                // Ensure we're using the progress bar style (in case we were in queued state before)
                                progress_bar.set_style(
                                    ProgressStyle::default_bar()
                                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})")
                                        .unwrap()
                                        .progress_chars("#>-")
                                );
                                
                                // Display progress information if available
                                if let Some(ref progress) = status.progress {
                                    // Only update if progress has changed
                                    if (progress.percentage - last_progress_percent).abs() > 0.1 {
                                        last_progress_percent = progress.percentage;
                                        
                                        // Update progress bar
                                        progress_bar.set_position(progress.percentage as u64);
                                        
                                        // Set message with detailed info
                                        progress_bar.set_message(format!(
                                            "Chunks: {}/{} | Duration: {:.1}/{:.1}s",
                                            progress.processed_chunks,
                                            progress.total_chunks,
                                            progress.processed_duration,
                                            progress.total_duration
                                        ));
                                    }
                                } else {
                                    // If no progress info, just pulse the bar
                                    progress_bar.set_message("Processing...".to_string());
                                    progress_bar.inc(0);
                                }
                            }
                            "queued" => {
                                // Show a pulsing progress bar for queued state
                                progress_bar.set_style(
                                    ProgressStyle::default_spinner()
                                        .template("{spinner:.yellow} {msg}")
                                        .unwrap()
                                );
                                progress_bar.set_message("Queued: waiting to be processed".yellow().to_string());
                                progress_bar.tick();
                            }
                            _ => {
                                // Unknown state, show warning in progress bar
                                progress_bar.set_message(format!("Unknown state: {}", status.status).yellow().to_string());
                                eprintln!("Job {} in unknown state: {}", job_id, status.status);
                            }
                        }
                    },
                    Err(e) => {
                        // If we can't get the status, the job might have been deleted
                        if e.to_string().contains("404") || e.to_string().contains("not found") {
                            progress_bar.abandon_with_message("Job no longer exists on server".red().to_string());
                            anyhow::bail!("Job no longer exists on server. It may have been terminated externally.");
                        } else {
                            // For other errors, log and continue
                            progress_bar.set_message(format!("Warning: {}", e).yellow().to_string());
                            eprintln!("Warning: Failed to get job status: {}", e);
                            // Continue polling, but don't fail immediately on temporary errors
                        }
                    }
                }
            }
        }
    }
}
