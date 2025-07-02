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
INSTALL_DIR="$HOME/.local/lib/waybar-modules"
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
    
    # Create example config file
    cat > "$EXAMPLE_DIR/config.json" << 'EOF'
{
    "layer": "top",
    "position": "top",
    "modules-center": ["cffi/virtual-desktops"],
    
    "cffi/virtual-desktops": {
        "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
        "format": "{name}",
        "show_empty": false,
        "separator": " ",
        "format_icons": {
            "1": "1",
            "2": "2", 
            "3": "3",
            "4": "4",
            "5": "5"
        },
        "show_window_count": true,
        "sort_by": "number"
    }
}
EOF
    
    # Create example CSS file
    cat > "$EXAMPLE_DIR/style.css" << 'EOF'
/* Virtual Desktop CFFI Module Styles */
#cffi-virtual-desktops {
    background-color: transparent;
    padding: 0 10px;
}

#cffi-virtual-desktops .vdesk-focused {
    background-color: #5e81ac;
    color: #eceff4;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
    font-weight: bold;
}

#cffi-virtual-desktops .vdesk-unfocused {
    background-color: #4c566a;
    color: #d8dee9;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
}

#cffi-virtual-desktops .vdesk-unfocused:hover {
    background-color: #5e81ac;
    color: #eceff4;
}

#cffi-virtual-desktops .hidden {
    display: none;
}
EOF
    
    print_success "Example configuration created in $EXAMPLE_DIR"
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
