use anyhow::{Context, Result};
use reqwest::multipart;
use std::path::PathBuf;
use std::time::Duration;
use crate::models::{JobResponse, TranscriptionResponse};
use crate::config::Config;

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

    if !response.status().is_success() {
        anyhow::bail!("Service health check failed");
    }

    Ok(())
}

pub async fn get_job_status(job_id: &str) -> Result<JobResponse> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/status/{}", CONFIG.service_url, job_id))
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

    let jobs: Vec<JobResponse> = response
        .json()
        .await
        .context("Failed to parse jobs response")?;

    Ok(jobs)
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
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if let Ok(terminated) = terminate_job(&job_id).await {
                    anyhow::bail!("Job terminated: {}", terminated.message);
                }
                anyhow::bail!("Job terminated by user");
            }
            _ = interval.tick() => {
                let status = get_job_status(&job_id).await?;

                match status.status.as_str() {
                    "completed" => {
                        if let Some(ref result) = status.result {
                            return Ok((result.clone(), status));
                        }
                        anyhow::bail!("No result in completed job");
                    }
                    "failed" => {
                        anyhow::bail!("Transcription failed: {}", status.message);
                    }
                    _ => {}
                }
            }
        }
    }
}