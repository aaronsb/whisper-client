# Whisper Client Usage Guide

This guide provides instructions on how to use the Whisper Client application for audio transcription.

## Commands

The Whisper Client supports the following commands:

### Transcribe

Transcribe an audio file or a directory of audio files:

```bash
whisper-client transcribe PATH [--recursive] [--verbose]
```

- `PATH`: Path to an audio file or directory containing audio files
- `--recursive` or `-r`: Process directory recursively (only valid with directory input)
- `--verbose` or `-v`: Show detailed output including segments

Example:
```bash
whisper-client transcribe recording.mp3 --verbose
whisper-client transcribe ./audio_files --recursive
```

### List Jobs

List all jobs on the Whisper service:

```bash
whisper-client list-jobs [--verbose]
```

- `--verbose` or `-v`: Show detailed job information

### Status

Get the status of a specific job:

```bash
whisper-client status --job-id JOB_ID [--verbose]
```

- `--job-id JOB_ID`: ID of the job to check
- `--verbose` or `-v`: Show detailed output including transcription if available

### Terminate

Terminate a specific job:

```bash
whisper-client terminate --job-id JOB_ID
```

- `--job-id JOB_ID`: ID of the job to terminate

## Job States

The Whisper Client handles the following job states:

- `queued`: Job is waiting to be processed
- `processing`: Job is currently being processed
- `completed`: Job has completed successfully
- `failed`: Job has failed
- `terminated`: Job was terminated by a user or the system
- `cancelled`: Job was cancelled

## External State Changes

The Whisper Client now properly responds to external state changes. If a job is terminated from the REST endpoint on the server, the client will detect this and exit gracefully with an appropriate error message.

## Error Handling

The client provides detailed error messages for various scenarios:

- Service connection issues
- Job not found
- Job terminated externally
- Transcription failures
- Invalid file formats
