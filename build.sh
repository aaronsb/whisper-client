#!/bin/bash

print_usage() {
    echo "Usage: $0 [--install]"
    echo "  --install    After successful build and test, install to /usr/local/bin"
    echo "  --help       Show this help message"
}

# Build the release version
build_and_test() {
    echo "Building release version..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi

    echo "Running tests..."
    cargo test
    if [ $? -ne 0 ]; then
        echo "Tests failed!"
        exit 1
    fi

    echo "Build and tests completed successfully!"
}

# Install the binary
install_binary() {
    echo "Installing to /usr/local/bin..."
    sudo cp target/release/whisper-client /usr/local/bin/
    if [ $? -ne 0 ]; then
        echo "Installation failed!"
        exit 1
    fi
    echo "Installation completed successfully!"
}

# Main script logic
if [ "$1" == "--help" ]; then
    print_usage
    exit 0
fi

build_and_test

if [ "$1" == "--install" ]; then
    install_binary
fi