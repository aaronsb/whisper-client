use std::path::PathBuf;

#[test]
fn test_m4a_mime_type() {
    let m4a_path = PathBuf::from("test.m4a");
    let mime = mime_guess::from_path(&m4a_path).first();
    
    assert!(mime.is_some(), "MIME type for m4a should be detected");
    
    let mime_str = mime.unwrap().to_string();
    println!("Detected MIME type for m4a: {}", mime_str);
    
    // Check if the MIME type is appropriate for m4a
    // Common MIME types for m4a include:
    // - audio/mp4
    // - audio/x-m4a
    // - audio/m4a
    assert!(
        mime_str == "audio/mp4" || 
        mime_str == "audio/x-m4a" || 
        mime_str == "audio/m4a",
        "Unexpected MIME type for m4a: {}", mime_str
    );
}
