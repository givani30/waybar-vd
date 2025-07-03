#!/bin/bash

# Installation script for Waybar Virtual Desktops CFFI Module
# This script provides an interactive installation experience

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="waybar-virtual-desktops-cffi"
LIB_NAME="libwaybar_virtual_desktops_cffi.so"
INSTALL_DIR="$HOME/.config/waybar/modules"
CONFIG_DIR="$HOME/.config/waybar"

# Functions
print_header() {
    clear
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}║           Waybar Virtual Desktops CFFI Module                ║${NC}"
    echo -e "${CYAN}║                    Installation Script                       ║${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
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

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

ask_yes_no() {
    local question="$1"
    local default="${2:-n}"
    local response
    
    if [ "$default" = "y" ]; then
        echo -n -e "${CYAN}$question [Y/n]: ${NC}"
    else
        echo -n -e "${CYAN}$question [y/N]: ${NC}"
    fi
    
    read response
    response=${response:-$default}
    
    case "$response" in
        [yY]|[yY][eE][sS]) return 0 ;;
        *) return 1 ;;
    esac
}

check_prerequisites() {
    print_step "Checking prerequisites..."
    local missing_deps=()
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("Rust/Cargo")
    fi
    
    # Check if Hyprland is available
    if ! command -v hyprctl &> /dev/null; then
        missing_deps+=("Hyprland")
    fi
    
    # Check if waybar is installed
    if ! command -v waybar &> /dev/null; then
        missing_deps+=("Waybar")
    fi
    
    # Check if waybar has CFFI support
    if command -v waybar &> /dev/null; then
        if ! waybar --version 2>/dev/null | grep -qi cffi; then
            print_warning "Waybar may not have CFFI support. Please ensure you have Waybar 0.12.0+ with CFFI enabled."
        fi
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        echo
        print_info "Please install the missing dependencies and run this script again."
        echo
        print_info "Installation guides:"
        print_info "- Rust: https://rustup.rs/"
        print_info "- Hyprland: https://hyprland.org/Installation/"
        print_info "- Waybar: https://github.com/Alexays/Waybar"
        exit 1
    fi
    
    print_success "All prerequisites are met"
}

backup_existing_config() {
    if [ -f "$CONFIG_DIR/config" ]; then
        if ask_yes_no "Backup existing Waybar configuration?"; then
            local backup_dir="$CONFIG_DIR/backup-$(date +%Y%m%d-%H%M%S)"
            mkdir -p "$backup_dir"
            
            if [ -f "$CONFIG_DIR/config" ]; then
                cp "$CONFIG_DIR/config" "$backup_dir/"
            fi
            
            if [ -f "$CONFIG_DIR/style.css" ]; then
                cp "$CONFIG_DIR/style.css" "$backup_dir/"
            fi
            
            print_success "Configuration backed up to $backup_dir"
        fi
    fi
}

build_and_install() {
    print_step "Building and installing the CFFI module..."
    
    # Use the build script
    if [ -f "./build.sh" ]; then
        ./build.sh
    else
        print_error "build.sh not found. Please run this script from the project directory."
        exit 1
    fi
}

setup_configuration() {
    print_step "Setting up configuration..."
    
    if ask_yes_no "Would you like to create an example configuration?"; then
        local example_dir="$CONFIG_DIR/examples/virtual-desktops-cffi"
        mkdir -p "$example_dir"
        
        # Check if example files were created by build.sh
        if [ -f "$example_dir/config.json" ]; then
            print_success "Example configuration already created in $example_dir"
        else
            print_warning "Example configuration not found. Please check the build output."
        fi
        
        echo
        print_info "Example configuration location: $example_dir"
        print_info "To use the example:"
        print_info "1. Copy config.json content to your waybar config"
        print_info "2. Copy style.css content to your waybar style.css"
        print_info "3. Restart waybar"
    fi
}

test_installation() {
    print_step "Testing installation..."
    
    # Check if library exists
    if [ -f "$INSTALL_DIR/$LIB_NAME" ]; then
        print_success "Library installed successfully"
    else
        print_error "Library not found at $INSTALL_DIR/$LIB_NAME"
        return 1
    fi
    
    # Check if test script is available
    local test_script="/home/givanib/dotfiles/config/waybar/test-cffi.sh"
    if [ -f "$test_script" ]; then
        if ask_yes_no "Would you like to run the test configuration?"; then
            print_info "Starting test configuration..."
            print_info "Press Ctrl+C in the test window to stop"
            echo
            "$test_script" --run
        fi
    else
        print_info "Test script not available. You can test manually by:"
        print_info "1. Adding the module to your waybar config"
        print_info "2. Restarting waybar"
        print_info "3. Checking for any error messages"
    fi
}

show_next_steps() {
    echo
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                     Installation Complete!                   ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo
    print_info "Next steps:"
    echo
    print_info "1. Add the module to your Waybar configuration:"
    echo -e "   ${CYAN}\"modules-center\": [\"cffi/virtual-desktops\"]${NC}"
    echo
    print_info "2. Configure the module:"
    echo -e "   ${CYAN}\"cffi/virtual-desktops\": {${NC}"
    echo -e "   ${CYAN}    \"library-path\": \"$INSTALL_DIR/$LIB_NAME\",${NC}"
    echo -e "   ${CYAN}    \"format\": \"{name}\"${NC}"
    echo -e "   ${CYAN}   }${NC}"
    echo
    print_info "3. Restart Waybar:"
    echo -e "   ${CYAN}pkill waybar && waybar &${NC}"
    echo
    print_info "4. Check the documentation:"
    print_info "   - README.md for detailed configuration options"
    print_info "   - MIGRATION.md if migrating from shell scripts"
    echo
    print_info "Library installed at: $INSTALL_DIR/$LIB_NAME"
    
    if [ -d "$CONFIG_DIR/examples/virtual-desktops-cffi" ]; then
        print_info "Example config at: $CONFIG_DIR/examples/virtual-desktops-cffi"
    fi
}

# Main installation flow
main() {
    print_header
    
    print_info "This script will install the Waybar Virtual Desktops CFFI Module"
    print_info "The installation includes:"
    print_info "- Building the Rust CFFI library"
    print_info "- Installing to ~/.config/waybar/modules/"
    print_info "- Creating example configurations"
    echo
    
    if ! ask_yes_no "Continue with installation?" "y"; then
        print_info "Installation cancelled."
        exit 0
    fi
    
    echo
    check_prerequisites
    echo
    
    backup_existing_config
    echo
    
    build_and_install
    echo
    
    setup_configuration
    echo
    
    test_installation
    echo
    
    show_next_steps
}

# Handle script arguments
case "${1:-}" in
    --help)
        echo "Waybar Virtual Desktops CFFI Module Installer"
        echo
        echo "Usage: $0 [OPTIONS]"
        echo
        echo "Options:"
        echo "  --help    Show this help message"
        echo
        echo "This script will guide you through the installation process."
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac
