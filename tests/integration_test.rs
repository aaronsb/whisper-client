use std::path::PathBuf;
use tempfile::tempdir;
use whisper_client::{is_supported_audio_format, collect_audio_files};

#[test]
fn test_m4a_integration() {
    // Verify m4a is supported
    let m4a_path = PathBuf::from("test.m4a");
    assert!(is_supported_audio_format(&m4a_path), "m4a should be supported");
    
    // Verify MIME type detection
    let mime = mime_guess::from_path(&m4a_path).first();
    assert!(mime.is_some(), "MIME type for m4a should be detected");
    
    let mime_str = mime.unwrap().to_string();
    println!("Detected MIME type for m4a: {}", mime_str);
    
    // Verify m4a files are collected correctly
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();
    
    // Create test files with different extensions including m4a
    std::fs::write(base_path.join("test1.mp3"), "dummy").unwrap();
    std::fs::write(base_path.join("test2.m4a"), "dummy").unwrap();
    
    // Collect audio files
    let files = collect_audio_files(&base_path.to_path_buf(), false).unwrap();
    
    // Verify m4a file is included in collected files
    let m4a_files: Vec<_> = files.iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("m4a"))
        .collect();
    
    assert_eq!(m4a_files.len(), 1, "Should find 1 m4a file");
    
    // Verify the path of the m4a file
    let expected_path = base_path.join("test2.m4a");
    assert!(m4a_files.contains(&&expected_path), "Should find the correct m4a file");
    
    println!("M4A file integration test passed successfully!");
}
