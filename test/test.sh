#!/bin/bash

# Virtual Desktop CFFI Module Test Script
# Self-contained testing within the Vd_waybar repository

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory (should be in Vd_waybar/test/)
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$TEST_DIR")"
LIBRARY_PATH="$REPO_DIR/target/release/libwaybar_virtual_desktops_cffi.so"
CONFIG_PATH="$TEST_DIR/waybar-config.json"
STYLE_PATH="$TEST_DIR/style.css"

echo -e "${BLUE}🧪 Virtual Desktop CFFI Module Test${NC}"
echo -e "${BLUE}====================================${NC}"
echo

# Function to print status
print_status() {
    local status=$1
    local message=$2
    case $status in
        "ok")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "warn")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "error")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "info")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

# Function to run prerequisite checks
check_prerequisites() {
    print_status "info" "Checking prerequisites..."
    
    # Check if we're in the right directory
    if [[ ! -f "$REPO_DIR/Cargo.toml" ]]; then
        print_status "error" "Not running from Vd_waybar repository directory"
        exit 1
    fi
    print_status "ok" "Running from correct repository directory"
    
    # Check if library exists
    if [[ ! -f "$LIBRARY_PATH" ]]; then
        print_status "warn" "CFFI library not found, building..."
        cd "$REPO_DIR"
        if cargo build --release; then
            print_status "ok" "CFFI library built successfully"
        else
            print_status "error" "Failed to build CFFI library"
            exit 1
        fi
        cd "$TEST_DIR"
    else
        print_status "ok" "CFFI library found: $(basename "$LIBRARY_PATH")"
    fi
    
    # Check if Hyprland is running
    if ! pgrep -x "Hyprland" > /dev/null; then
        print_status "error" "Hyprland is not running"
        exit 1
    fi
    print_status "ok" "Hyprland is running"
    
    # Check if virtual desktop plugin is loaded
    if ! hyprctl plugins list 2>/dev/null | grep -q "virtual-desktops"; then
        print_status "error" "Hyprland virtual-desktops plugin not loaded"
        print_status "info" "Please install and load the virtual-desktops plugin"
        exit 1
    fi
    print_status "ok" "Virtual desktops plugin is loaded"
    
    # Check if waybar is available
    if ! command -v waybar &> /dev/null; then
        print_status "error" "Waybar is not installed"
        exit 1
    fi
    print_status "ok" "Waybar is available: $(waybar --version 2>&1 | head -n1)"
    
    # Check for CFFI support in waybar
    if ! waybar --help 2>&1 | grep -q "cffi\|CFFI"; then
        print_status "warn" "Waybar CFFI support may not be available"
        print_status "info" "Continuing anyway - will test compatibility"
    else
        print_status "ok" "Waybar appears to support CFFI modules"
    fi
    
    echo
}

# Function to validate configuration
validate_config() {
    print_status "info" "Validating test configuration..."
    
    # Check if config file exists and is valid JSON
    if [[ ! -f "$CONFIG_PATH" ]]; then
        print_status "error" "Test config file not found: $CONFIG_PATH"
        exit 1
    fi
    
    if ! jq empty "$CONFIG_PATH" 2>/dev/null; then
        print_status "error" "Invalid JSON in config file"
        exit 1
    fi
    print_status "ok" "Configuration file is valid JSON"
    
    # Check if style file exists
    if [[ ! -f "$STYLE_PATH" ]]; then
        print_status "error" "Test style file not found: $STYLE_PATH"
        exit 1
    fi
    print_status "ok" "Style file exists"
    
    # Check library path in config
    local config_lib_path=$(jq -r '."cffi/virtual_desktops".module_path' "$CONFIG_PATH")
    local resolved_path

    # Handle both absolute and relative paths
    if [[ "$config_lib_path" = /* ]]; then
        # Absolute path
        resolved_path="$config_lib_path"
    else
        # Relative path
        resolved_path="$TEST_DIR/$config_lib_path"
    fi

    if [[ ! -f "$resolved_path" ]]; then
        print_status "error" "Library path in config is incorrect: $config_lib_path"
        print_status "info" "Expected: $resolved_path"
        exit 1
    fi
    print_status "ok" "Library path in config is correct"
    
    echo
}

# Function to run the test
run_test() {
    print_status "info" "Starting Waybar test instance..."
    print_status "info" "Press Ctrl+C to stop the test"
    echo
    
    # Set environment variables for logging
    export RUST_LOG=debug
    export G_MESSAGES_DEBUG=all
    
    # Kill any existing test waybar instances
    pkill -f "waybar.*$CONFIG_PATH" || true
    sleep 1
    
    # Start waybar with test configuration
    print_status "info" "Command: waybar --log-level debug -c \"$CONFIG_PATH\" -s \"$STYLE_PATH\""
    print_status "info" "Log output will show below (debug mode enabled):"
    echo -e "${YELLOW}===============================================${NC}"
    
    # Run waybar in foreground so we can see output and easily stop it
    waybar -c "$CONFIG_PATH" -s "$STYLE_PATH" || {
        print_status "error" "Waybar test failed"
        return 1
    }
}

# Function to show help
show_help() {
    echo "Virtual Desktop CFFI Module Test Script"
    echo
    echo "Usage: $0 [COMMAND]"
    echo
    echo "Commands:"
    echo "  --check     Check prerequisites only"
    echo "  --dry-run   Validate configuration without running"
    echo "  --run       Run the full test (default)"
    echo "  --build     Build the CFFI library only"
    echo "  --help      Show this help message"
    echo
    echo "This script tests the virtual desktop CFFI module in isolation."
    echo "It runs a separate waybar instance that won't interfere with your main setup."
    echo
}

# Function to build only
build_only() {
    print_status "info" "Building CFFI library..."
    cd "$REPO_DIR"
    if cargo build --release; then
        print_status "ok" "Build successful: $LIBRARY_PATH"
        print_status "info" "Library size: $(du -h "$LIBRARY_PATH" | cut -f1)"
    else
        print_status "error" "Build failed"
        exit 1
    fi
}

# Main execution
case "${1:-}" in
    "--help")
        show_help
        ;;
    "--check")
        check_prerequisites
        print_status "ok" "All prerequisites satisfied"
        ;;
    "--dry-run")
        check_prerequisites
        validate_config
        print_status "ok" "Configuration is valid and ready for testing"
        ;;
    "--build")
        build_only
        ;;
    "--run"|"")
        check_prerequisites
        validate_config
        run_test
        ;;
    *)
        print_status "error" "Unknown command: $1"
        show_help
        exit 1
        ;;
esac