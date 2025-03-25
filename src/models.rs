use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Progress {
    pub total_duration: f64,
    pub processed_duration: f64,
    pub total_chunks: i32,
    pub processed_chunks: i32,
    pub percentage: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Segment {
    pub id: i32,
    pub seek: i32,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub tokens: Vec<i64>,
    pub temperature: f64,
    pub avg_logprob: f64,
    pub compression_ratio: f64,
    pub no_speech_prob: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TranscriptionResponse {
    pub text: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JobResponse {
    pub job_id: String,
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<TranscriptionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_info: Option<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Progress>,
}
