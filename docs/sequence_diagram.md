# Sequence Diagram for YouTube Transcription Process

```mermaid
sequenceDiagram
    participant User
    participant Client
    participant YouTubeDownloader as yt-dlp
    participant WhisperService as Whisper Service

    User->>Client: Provide YouTube URL
    Client->>YouTubeDownloader: Download Video
    alt Download Successful
        YouTubeDownloader-->>Client: Video File
        Client->>Client: Convert Video to Audio
        alt Conversion Successful
            Client->>WhisperService: Transcribe Audio
            alt Transcription Successful
                WhisperService-->>Client: Transcription Result
                Client-->>User: Display Transcription
            else Transcription Failed
                WhisperService-->>Client: Error Message
                Client-->>User: Display Error
            end
        else Conversion Failed
            Client-->>User: Display Conversion Error
        end
    else Download Failed
        YouTubeDownloader-->>Client: Error Message
        Client-->>User: Display Download Error
    end
```

## Description

- **User**: Initiates the transcription process by providing a YouTube URL.
- **Client**: Manages the overall process, including downloading, converting, and transcribing.
- **yt-dlp**: Downloads the video from YouTube.
- **Whisper Service**: Transcribes the audio to text.
- The process includes error handling at each stage, with appropriate messages displayed to the user.
