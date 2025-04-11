use tempfile::tempdir;
use whisper_client::collect_audio_files;

#[test]
fn test_collect_audio_files_with_m4a() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create test files with different extensions
    std::fs::write(base_path.join("test1.mp3"), "dummy").unwrap();
    std::fs::write(base_path.join("test2.wav"), "dummy").unwrap();
    std::fs::write(base_path.join("test3.m4a"), "dummy").unwrap();
    std::fs::write(base_path.join("test4.txt"), "dummy").unwrap();

    // Create a subdirectory with more files
    let sub_dir = base_path.join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("test5.mp3"), "dummy").unwrap();
    std::fs::write(sub_dir.join("test6.m4a"), "dummy").unwrap();

    // Test non-recursive collection
    let files = collect_audio_files(&base_path.to_path_buf(), false).unwrap();
    assert_eq!(files.len(), 3, "Should find 3 audio files in base directory (mp3, wav, m4a)");
    
    // Verify m4a file is included
    let m4a_files: Vec<_> = files.iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("m4a"))
        .collect();
    assert_eq!(m4a_files.len(), 1, "Should find 1 m4a file in base directory");

    // Test recursive collection
    let files = collect_audio_files(&base_path.to_path_buf(), true).unwrap();
    assert_eq!(files.len(), 5, "Should find 5 audio files in total (3 in base + 2 in subdir)");
    
    // Verify m4a files are included
    let m4a_files: Vec<_> = files.iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("m4a"))
        .collect();
    assert_eq!(m4a_files.len(), 2, "Should find 2 m4a files in total");

    // Test single m4a file
    let single_file = base_path.join("test3.m4a");
    let files = collect_audio_files(&single_file, false).unwrap();
    assert_eq!(files.len(), 1, "Should handle single m4a file");
    assert_eq!(files[0], single_file, "Should return the correct m4a file path");
}
