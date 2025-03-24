#!/bin/bash

print_usage() {
    echo "Usage: $0 [--clean] [--build] [--install]"
    echo "  --clean      Clean up the build artifacts"
    echo "  --build      Build the release version and run tests"
    echo "  --install    After successful build, install to ~/.local/bin"
    echo "  --help       Show this help message"
    echo ""
    echo "If no options are provided, this help message will be displayed."
    echo "Actions will be performed in this order: clean, build, install (if specified)."
}

# Clean up the build
clean_build() {
    echo "Cleaning up build artifacts..."
    cargo clean
    if [ $? -ne 0 ]; then
        echo "Clean failed!"
        exit 1
    fi
    echo "Clean completed successfully!"
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
    echo "Installing to ~/.local/bin..."
    # Ensure the directory exists
    mkdir -p ~/.local/bin
    cp target/release/whisper-client ~/.local/bin/
    if [ $? -ne 0 ]; then
        echo "Installation failed!"
        exit 1
    fi
    echo "Installation completed successfully!"
}

# Main script logic
CLEAN=false
BUILD=false
INSTALL=false

# Parse command line arguments
for arg in "$@"; do
    case $arg in
        --clean)
            CLEAN=true
            ;;
        --build)
            BUILD=true
            ;;
        --install)
            INSTALL=true
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            print_usage
            exit 1
            ;;
    esac
done

# If no arguments provided, show help
if [ $# -eq 0 ]; then
    print_usage
    exit 0
fi

# Execute actions in the correct order
if [ "$CLEAN" = true ]; then
    clean_build
fi

if [ "$BUILD" = true ]; then
    build_and_test
fi

if [ "$INSTALL" = true ]; then
    # Check if we need to build first if not already built
    if [ "$BUILD" = false ]; then
        if [ ! -f "target/release/whisper-client" ]; then
            echo "No build found. Building before install..."
            build_and_test
        fi
    fi
    install_binary
fi

echo "All requested operations completed."
