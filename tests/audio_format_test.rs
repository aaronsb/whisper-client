use std::path::PathBuf;
use whisper_client::is_supported_audio_format;

#[test]
fn test_supported_audio_formats() {
    // Test all supported formats
    let formats = vec![
        ("test.mp3", true),
        ("test.wav", true),
        ("test.m4a", true),
        ("test.ogg", true),
        ("test.flac", true),
        ("test.mkv", true),
        ("test.mp4", true),
        ("test.txt", false),
        ("test.pdf", false),
        ("test", false),
        // Test case variations
        ("test.M4A", true),
        ("test.M4a", true),
        ("TEST.M4A", true),
        // Test with paths
        ("path/to/audio.m4a", true),
        ("/absolute/path/to/audio.m4a", true),
        // Test with spaces and special characters
        ("my audio file.m4a", true),
        ("audio-file_123.m4a", true),
    ];

    for (file, expected) in formats {
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
fn test_mime_types() {
    // Test MIME type detection for all supported formats
    let formats = vec![
        ("test.mp3", vec!["audio/mpeg"]),
        ("test.wav", vec!["audio/wav", "audio/x-wav", "audio/wave"]),
        ("test.m4a", vec!["audio/mp4", "audio/x-m4a", "audio/m4a"]),
        ("test.ogg", vec!["audio/ogg", "application/ogg"]),
        ("test.flac", vec!["audio/flac", "audio/x-flac"]),
        ("test.mkv", vec!["video/x-matroska"]),
        ("test.mp4", vec!["video/mp4"]),
    ];

    for (file, expected_mimes) in formats {
        let path = PathBuf::from(file);
        let mime = mime_guess::from_path(&path).first();
        
        assert!(mime.is_some(), "MIME type for {} should be detected", file);
        
        let mime_str = mime.unwrap().to_string();
        println!("Detected MIME type for {}: {}", file, mime_str);
        
        assert!(
            expected_mimes.contains(&mime_str.as_str()),
            "Unexpected MIME type for {}: {}, expected one of: {:?}", 
            file, mime_str, expected_mimes
        );
    }
}
