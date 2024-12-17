use anyhow::{Context, Result};
use std::path::PathBuf;
use crate::models::{TranscriptionResponse, JobResponse};

pub fn is_supported_audio_format(path: &PathBuf) -> bool {
    let supported = ["mp3", "wav", "m4a", "ogg", "flac"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn collect_audio_files(path: &PathBuf, recursive: bool) -> Result<Vec<PathBuf>> {
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

pub fn save_markdown_response(
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_supported_audio_formats() {
        let test_files = vec![
            ("test.mp3", true),
            ("test.wav", true),
            ("test.m4a", true),
            ("test.ogg", true),
            ("test.flac", true),
            ("test.txt", false),
            ("test.pdf", false),
            ("test", false),
        ];

        for (file, expected) in test_files {
            let path = PathBuf::from(file);
            assert_eq!(
                is_supported_audio_format(&path),
                expected,
                "Failed for file: {}",
                file
            );
        }
    }

    #[test]
    fn test_collect_audio_files() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path();

        // Create test files
        std::fs::write(base_path.join("test1.mp3"), "dummy").unwrap();
        std::fs::write(base_path.join("test2.wav"), "dummy").unwrap();
        std::fs::write(base_path.join("test3.txt"), "dummy").unwrap();

        // Create a subdirectory with more files
        let sub_dir = base_path.join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("test4.mp3"), "dummy").unwrap();
        std::fs::write(sub_dir.join("test5.wav"), "dummy").unwrap();

        // Test non-recursive collection
        let files = collect_audio_files(&base_path.to_path_buf(), false).unwrap();
        assert_eq!(files.len(), 2, "Should find 2 audio files in base directory");

        // Test recursive collection
        let files = collect_audio_files(&base_path.to_path_buf(), true).unwrap();
        assert_eq!(files.len(), 4, "Should find 4 audio files in total");

        // Test single file
        let single_file = base_path.join("test1.mp3");
        let files = collect_audio_files(&single_file, false).unwrap();
        assert_eq!(files.len(), 1, "Should handle single file");
    }

    #[test]
    fn test_save_markdown_response() {
        use crate::models::{FileInfo, JobResponse, Segment, TranscriptionResponse};
        
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("test_audio.mp3");
        
        let response = TranscriptionResponse {
            text: String::from("This is a test transcription."),
            segments: vec![
                Segment {
                    id: 0,
                    seek: 0,
                    start: 0.0,
                    end: 2.5,
                    text: String::from("This is"),
                    tokens: vec![1, 2],
                    temperature: 0.0,
                    avg_logprob: -0.5,
                    compression_ratio: 1.0,
                    no_speech_prob: 0.1,
                },
                Segment {
                    id: 1,
                    seek: 100,
                    start: 2.5,
                    end: 5.0,
                    text: String::from("a test transcription."),
                    tokens: vec![3, 4, 5],
                    temperature: 0.0,
                    avg_logprob: -0.5,
                    compression_ratio: 1.0,
                    no_speech_prob: 0.1,
                },
            ],
        };

        let job_info = JobResponse {
            job_id: String::from("test-job"),
            status: String::from("completed"),
            message: String::from(""),
            result: Some(response.clone()),
            file_info: Some(FileInfo {
                name: String::from("test_audio.mp3"),
                size: 1000,
            }),
            created_at: Some(1234567890.0),
            filename: Some(String::from("test_audio.mp3")),
        };

        let output_path = save_markdown_response(
            &response,
            &input_path,
            &job_info,
        ).unwrap();

        // Verify the output file exists and has the correct extension
        assert!(output_path.exists());
        assert_eq!(output_path.extension().unwrap(), "md");

        // Read and verify the content
        let content = std::fs::read_to_string(output_path).unwrap();
        assert!(content.contains("This is a test transcription."));
        assert!(content.contains("## Audio File Information"));
        assert!(content.contains("- **Source File:**"));
        assert!(content.contains("- **File Size:** 1000 bytes"));
        assert!(content.contains("- **Duration:** 0:05"));
    }
}