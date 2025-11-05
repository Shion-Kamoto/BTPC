#!/bin/bash

# BTPC Desktop UI Launch Script - Robust Version
# This script sets up and runs the BTPC desktop application

# Exit on error for critical setup, but handle build errors gracefully
set -e

echo "🚀 Starting BTPC Desktop UI..."
echo "================================"

# Check if we're in the right directory
if [ ! -f "btpc-ui/src-tauri/Cargo.toml" ]; then
    echo "❌ Error: Run this script from the BTPC project root directory"
    echo "   Current directory should contain 'btpc-ui/' folder"
    echo "   Current directory: $(pwd)"
    echo "   Looking for: btpc-ui/src-tauri/Cargo.toml"
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check if Node.js is installed
if ! command -v npm &> /dev/null; then
    echo "❌ Error: Node.js/npm not found. Please install Node.js from https://nodejs.org/"
    exit 1
fi

# Function to setup frontend
setup_frontend() {
    echo "📦 Setting up frontend..."

    # Install dependencies if needed
    if [ ! -d "btpc-ui/frontend/node_modules" ]; then
        echo "   Installing npm packages..."
        cd btpc-ui/frontend
        npm install
        cd ../..
    else
        echo "   Dependencies already installed"
    fi
}

# Function to build frontend
build_frontend() {
    echo "🏗️  Building frontend..."
    cd btpc-ui/frontend
    npm run build
    cd ../..
}

# Function to check tauri cli
check_tauri() {
    if ! command -v cargo-tauri &> /dev/null && ! cargo tauri --version &> /dev/null 2>&1; then
        echo "📥 Installing Tauri CLI..."
        cargo install tauri-cli --version "^2.0"
    else
        echo "✅ Tauri CLI found"
    fi
}

# Main execution
setup_frontend

# Check and create icons if needed
create_icon_if_missing() {
    if [ ! -f "btpc-ui/src-tauri/icons/icon.png" ]; then
        echo "⚠️  Creating RGBA icons for Tauri..."
        mkdir -p btpc-ui/src-tauri/icons
        # Create proper RGBA PNG icons using Python
        python3 -c "
from PIL import Image
import os
os.chdir('btpc-ui/src-tauri/icons')
# Create BTPC blue RGBA icons
img_32 = Image.new('RGBA', (32, 32), (30, 58, 138, 255))
img_32.save('icon.png')
img_32.save('32x32.png')
img_128 = Image.new('RGBA', (128, 128), (30, 58, 138, 255))
img_128.save('128x128.png')
img_256 = Image.new('RGBA', (256, 256), (30, 58, 138, 255))
img_256.save('128x128@2x.png')
" 2>/dev/null || echo "   Note: Could not create icons automatically"
    fi
}

# Check if in development mode
if [ "$1" = "--dev" ]; then
    echo "🔧 Running in development mode with hot reload..."
    echo "   Frontend dev server will start automatically on http://localhost:1420"
    echo "   Tauri will wait for the frontend to be ready..."
    check_tauri
    create_icon_if_missing

    # Ensure frontend dependencies are installed
    echo "📦 Installing frontend dependencies..."
    cd btpc-ui/frontend
    if [ ! -d "node_modules" ] || [ ! -f "node_modules/.package-lock.json" ]; then
        npm install
    fi
    cd ../../

    echo "🚀 Starting Tauri dev mode..."
    echo "   (This will automatically start the Vite dev server and then launch Tauri)"
    echo ""
    echo "📝 Note: The first time may take a few minutes to:"
    echo "   - Start Vite dev server on http://localhost:1420"
    echo "   - Compile Rust backend"
    echo "   - Launch the desktop application"
    echo ""

    cd btpc-ui/src-tauri

    # Handle potential compilation errors gracefully
    set +e  # Disable exit on error for build command
    cargo tauri dev
    BUILD_RESULT=$?
    set -e  # Re-enable exit on error

    if [ $BUILD_RESULT -ne 0 ]; then
        echo ""
        echo "❌ Development build failed. Common issues:"
        echo "   - Check that all dependencies are installed"
        echo "   - Try running 'npm run dev' in btpc-ui/frontend/ to test the frontend"
        echo "   - Try running 'cargo check' in btpc-ui/src-tauri/ for detailed errors"
        echo "   - Make sure port 1420 is not in use by another process"
        echo ""
        echo "🔧 Debug steps:"
        echo "   1. Test frontend: cd btpc-ui/frontend && npm run dev"
        echo "   2. Test backend: cd btpc-ui/src-tauri && cargo check"
        exit 1
    fi
else
    # Production build
    build_frontend
    check_tauri
    create_icon_if_missing

    echo "🖥️  Launching BTPC Desktop UI..."
    echo "   - Network: Configurable (Mainnet/Testnet/Regtest)"
    echo "   - Features: Wallet, Mining, Explorer, Settings"
    echo "   - Security: Quantum-resistant ML-DSA-87 signatures"
    echo ""

    echo "⚡ Building optimized release version..."
    cd btpc-ui/src-tauri

    set +e  # Disable exit on error for build command
    cargo tauri build
    BUILD_RESULT=$?
    set -e  # Re-enable exit on error

    if [ $BUILD_RESULT -ne 0 ]; then
        echo ""
        echo "❌ Production build failed. Common issues:"
        echo "   - Check that all dependencies are installed"
        echo "   - Try running 'cargo check' in btpc-ui/src-tauri/ for detailed errors"
        echo "   - Make sure the frontend built successfully"
        exit 1
    fi

    echo ""
    echo "✅ Build complete!"
    echo "📦 Installer location: target/release/bundle/"
    echo "🎯 Run the app from: target/release/btpc-ui"

    # Ask if user wants to run the app
    echo ""
    read -p "🚀 Run the application now? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "▶️  Starting BTPC Desktop UI..."
        if [ -f "target/release/btpc-ui" ]; then
            ./target/release/btpc-ui
        else
            echo "❌ Build executable not found. Please check the build output above."
        fi
    fi
fi