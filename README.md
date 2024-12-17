# Whisper Client

A command-line client for transcribing audio files using OpenAI's Whisper model via the [whisper-service](https://github.com/aaronsb/whisper-service) backend.

## Features

- Transcribe audio files to text using Whisper
- Support for batch processing directories of audio files
- Track transcription job status
- View job history
- Configurable service endpoint
- Markdown output format

## Installation

### Using Build Scripts

#### Linux
```bash
# Build and test
./build.sh

# Build, test, and install to /usr/local/bin
./build.sh --install
```

#### Windows
```powershell
# Build and test
.\build.ps1

# Build, test, and install to Program Files
.\build.ps1 -Install
```

### Manual Installation

```bash
# Build from source
cargo build --release

# Copy to a location in your PATH
sudo cp target/release/whisper-client /usr/local/bin/
```

## Configuration

On first run, the client creates a configuration file at:
- Linux: `~/.config/whisper-client/config.json`
- Windows: `%USERPROFILE%\.config\whisper-client\config.json`

Default configuration:
```json
{
  "service_url": "http://localhost:8000"
}
```

Edit this file to point to your whisper-service instance if it's running on a different host or port.

## Usage

### Transcribe a Single File
```bash
whisper-client transcribe PATH_TO_FILE
```

### Transcribe a Directory
```bash
# Process all audio files in directory
whisper-client transcribe PATH_TO_DIR

# Process directory recursively
whisper-client transcribe -r PATH_TO_DIR
```

### List All Jobs
```bash
whisper-client list-jobs

# With detailed information
whisper-client list-jobs -v
```

### Check Job Status
```bash
whisper-client status --job-id JOB_ID

# With transcription output (if completed)
whisper-client status --job-id JOB_ID -v
```

## Output

Transcriptions are saved as markdown files next to the source audio files, containing:
- Transcribed text
- Timestamps for each segment
- Job metadata

## Service Requirements

This client requires a running instance of [whisper-service](https://github.com/aaronsb/whisper-service). By default, it expects the service to be running at `http://localhost:8000`. You can modify the service URL in the configuration file.

## Supported Audio Formats

The client supports common audio formats including:
- WAV
- MP3
- M4A
- FLAC
- OGG

## Error Handling

- Automatically retries failed connections
- Graceful handling of service interruptions
- Clear error messages for common issues
- CTRL+C support for canceling running jobs

## Development

Built with Rust, using:
- clap for CLI argument parsing
- reqwest for HTTP requests
- tokio for async runtime
- serde for JSON handling

## License

MIT License