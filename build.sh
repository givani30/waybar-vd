#!/bin/bash

# Build script for Waybar Virtual Desktops CFFI Module
# This script builds the CFFI shared library for Waybar

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="waybar-virtual-desktops-cffi"
LIB_NAME="libwaybar_virtual_desktops_cffi.so"
INSTALL_DIR="$HOME/.config/waybar/modules"
CONFIG_DIR="$HOME/.config/waybar"

# Functions
print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  Waybar Virtual Desktops CFFI  ${NC}"
    echo -e "${BLUE}================================${NC}"
    echo
}

print_step() {
    echo -e "${YELLOW}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

check_prerequisites() {
    print_step "Checking prerequisites..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo (Rust) is not installed. Please install Rust first."
        exit 1
    fi
    
    # Check if Hyprland virtual desktop plugin is available
    if ! command -v hyprctl &> /dev/null; then
        print_error "hyprctl not found. Make sure Hyprland is installed."
        exit 1
    fi
    
    # Check if waybar is installed
    if ! command -v waybar &> /dev/null; then
        print_error "Waybar is not installed. Please install Waybar first."
        exit 1
    fi
    
    print_success "All prerequisites are met"
}

build_library() {
    print_step "Building CFFI shared library..."
    
    # Clean previous builds
    cargo clean
    
    # Build in release mode
    cargo build --release
    # Strip debug symbols
    strip target/release/libwaybar_virtual_desktops_cffi.so
    if [ $? -eq 0 ]; then
        print_success "Library built successfully"
    else
        print_error "Build failed"
        exit 1
    fi
}

install_library() {
    print_step "Installing library..."
    
    # Create installation directory
    mkdir -p "$INSTALL_DIR"
    
    # Copy the shared library
    cp "target/release/$LIB_NAME" "$INSTALL_DIR/"
    
    if [ $? -eq 0 ]; then
        print_success "Library installed to $INSTALL_DIR/$LIB_NAME"
    else
        print_error "Installation failed"
        exit 1
    fi
}

create_example_config() {
    print_step "Creating example configuration..."
    
    # Create example config directory
    EXAMPLE_DIR="$CONFIG_DIR/examples/virtual-desktops-cffi"
    mkdir -p "$EXAMPLE_DIR"
    
    # Copy example files from project examples directory
    if [ -f "./examples/config.json" ]; then
        cp "./examples/config.json" "$EXAMPLE_DIR/"
        print_success "Example config copied to $EXAMPLE_DIR"
    else
        print_error "Example config.json not found in ./examples/"
    fi
    
    if [ -f "./examples/style.css" ]; then
        cp "./examples/style.css" "$EXAMPLE_DIR/"
        print_success "Example CSS copied to $EXAMPLE_DIR"
    else
        print_error "Example style.css not found in ./examples/"
    fi
}

show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --build-only    Only build the library, don't install"
    echo "  --install-only  Only install (assumes library is already built)"
    echo "  --clean         Clean build artifacts"
    echo "  --help          Show this help message"
    echo
    echo "Default: Build and install the library with example configuration"
}

clean_build() {
    print_step "Cleaning build artifacts..."
    cargo clean
    print_success "Build artifacts cleaned"
}

# Main execution
main() {
    print_header
    
    case "${1:-}" in
        --build-only)
            check_prerequisites
            build_library
            ;;
        --install-only)
            install_library
            create_example_config
            ;;
        --clean)
            clean_build
            ;;
        --help)
            show_usage
            ;;
        "")
            check_prerequisites
            build_library
            install_library
            create_example_config
            
            echo
            print_info "Installation complete!"
            print_info "Example configuration available in: $EXAMPLE_DIR"
            print_info "Library installed in: $INSTALL_DIR/$LIB_NAME"
            echo
            print_info "Next steps:"
            print_info "1. Copy the example config to your waybar configuration"
            print_info "2. Restart waybar to load the new module"
            print_info "3. Check the README.md for detailed configuration options"
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
}

main "$@"
