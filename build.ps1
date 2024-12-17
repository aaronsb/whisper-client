# PowerShell script for building and installing whisper-client

function Print-Usage {
    Write-Host "Usage: .\build.ps1 [-Install]"
    Write-Host "  -Install    After successful build and test, install to Program Files"
    Write-Host "  -Help       Show this help message"
}

function Build-And-Test {
    Write-Host "Building release version..."
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed!"
        exit 1
    }

    Write-Host "Running tests..."
    cargo test
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Tests failed!"
        exit 1
    }

    Write-Host "Build and tests completed successfully!"
}

function Install-Binary {
    $installDir = "$env:ProgramFiles\WhisperClient"
    
    Write-Host "Installing to $installDir..."
    
    # Create install directory if it doesn't exist
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force
    }
    
    # Copy the binary
    Copy-Item "target\release\whisper-client.exe" -Destination $installDir -Force
    
    # Add to PATH if not already present
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if ($currentPath -notlike "*$installDir*") {
        $newPath = "$currentPath;$installDir"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
        Write-Host "Added installation directory to system PATH"
    }
    
    Write-Host "Installation completed successfully!"
}

# Main script logic
param(
    [switch]$Install,
    [switch]$Help
)

if ($Help) {
    Print-Usage
    exit 0
}

Build-And-Test

if ($Install) {
    Install-Binary
}