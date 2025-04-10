# State Diagram for Whisper Client

```mermaid
stateDiagram
    [*] --> Idle
    Idle --> Downloading : Start YouTube Transcription
    Downloading --> Converting : Download Complete
    Converting --> Transcribing : Conversion Complete
    Transcribing --> Completed : Transcription Complete
    Transcribing --> Error : Transcription Failed
    Downloading --> Error : Download Failed
    Converting --> Error : Conversion Failed
    Error --> [*]
    Completed --> [*]
```

## Description

- **Idle**: The client is waiting for a command.
- **Downloading**: The client is downloading the YouTube video.
- **Converting**: The client is converting the video to audio format.
- **Transcribing**: The client is transcribing the audio to text.
- **Completed**: The transcription process is successfully completed.
- **Error**: An error occurred during any of the stages.
