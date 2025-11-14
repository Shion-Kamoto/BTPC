#!/bin/bash
# BTPC Pieces MCP Setup Script
# Automates Pieces MCP installation and configuration for Claude Code

set -e  # Exit on error

echo "🔧 BTPC Pieces MCP Setup"
echo "======================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Pieces OS is installed
check_pieces_os() {
    echo "📦 Checking for Pieces OS..."

    if command -v pieces-os &> /dev/null; then
        echo -e "${GREEN}✓ Pieces OS found${NC}"
        return 0
    fi

    if [ -f "$HOME/pieces-os.AppImage" ]; then
        echo -e "${GREEN}✓ Pieces OS AppImage found${NC}"
        return 0
    fi

    echo -e "${YELLOW}⚠ Pieces OS not found${NC}"
    return 1
}

# Check if Pieces OS is running
check_pieces_running() {
    echo "🔍 Checking if Pieces OS is running..."

    if curl -s http://localhost:1000/health &> /dev/null; then
        echo -e "${GREEN}✓ Pieces OS is running${NC}"
        return 0
    fi

    echo -e "${YELLOW}⚠ Pieces OS is not running${NC}"
    return 1
}

# Start Pieces OS
start_pieces_os() {
    echo "🚀 Starting Pieces OS..."

    if [ -f "$HOME/pieces-os.AppImage" ]; then
        "$HOME/pieces-os.AppImage" &> /dev/null &
        sleep 5  # Wait for Pieces OS to start

        if curl -s http://localhost:1000/health &> /dev/null; then
            echo -e "${GREEN}✓ Pieces OS started successfully${NC}"
            return 0
        fi
    fi

    echo -e "${RED}✗ Failed to start Pieces OS${NC}"
    return 1
}

# Install Pieces CLI
install_pieces_cli() {
    echo "📥 Installing Pieces CLI..."

    if command -v pieces &> /dev/null; then
        echo -e "${GREEN}✓ Pieces CLI already installed${NC}"
        pieces --version
        return 0
    fi

    if command -v npm &> /dev/null; then
        npm install -g @pieces.app/cli
        echo -e "${GREEN}✓ Pieces CLI installed${NC}"
        return 0
    else
        echo -e "${RED}✗ npm not found. Please install Node.js first.${NC}"
        return 1
    fi
}

# Run Pieces MCP setup
run_pieces_mcp_setup() {
    echo "🔧 Running Pieces MCP setup for Claude..."

    if ! command -v pieces &> /dev/null; then
        echo -e "${RED}✗ Pieces CLI not found${NC}"
        return 1
    fi

    # Run the setup command
    pieces mcp setup --claude

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Pieces MCP configured for Claude${NC}"
        return 0
    else
        echo -e "${RED}✗ Pieces MCP setup failed${NC}"
        return 1
    fi
}

# Add permissions to Claude settings
add_permissions() {
    echo "🔐 Adding Pieces MCP permissions..."

    SETTINGS_FILE=".claude/settings.local.json"

    if [ ! -f "$SETTINGS_FILE" ]; then
        echo -e "${YELLOW}⚠ Settings file not found: $SETTINGS_FILE${NC}"
        return 1
    fi

    # Check if Pieces permissions already exist
    if grep -q "mcp__pieces__" "$SETTINGS_FILE"; then
        echo -e "${GREEN}✓ Pieces permissions already configured${NC}"
        return 0
    fi

    echo "Adding Pieces MCP permissions to $SETTINGS_FILE..."

    # Backup original file
    cp "$SETTINGS_FILE" "$SETTINGS_FILE.backup"

    # Note: Manual addition recommended due to JSON complexity
    echo -e "${YELLOW}⚠ Please manually add Pieces permissions to $SETTINGS_FILE${NC}"
    echo "Add these lines to the \"allow\" array:"
    echo "  \"mcp__pieces__save_snippet\","
    echo "  \"mcp__pieces__search_snippets\","
    echo "  \"mcp__pieces__get_snippet\","
    echo "  \"mcp__pieces__get_context\""

    return 0
}

# Verify installation
verify_installation() {
    echo ""
    echo "🔍 Verifying installation..."
    echo ""

    # Check Pieces OS
    if check_pieces_running; then
        echo -e "${GREEN}✓ Pieces OS: Running${NC}"
    else
        echo -e "${RED}✗ Pieces OS: Not running${NC}"
    fi

    # Check Pieces CLI
    if command -v pieces &> /dev/null; then
        echo -e "${GREEN}✓ Pieces CLI: Installed ($(pieces --version))${NC}"
    else
        echo -e "${RED}✗ Pieces CLI: Not installed${NC}"
    fi

    # Check MCP configuration
    if [ -f "$HOME/.config/claude/claude_desktop_config.json" ]; then
        if grep -q "pieces" "$HOME/.config/claude/claude_desktop_config.json"; then
            echo -e "${GREEN}✓ MCP Config: Pieces configured${NC}"
        else
            echo -e "${YELLOW}⚠ MCP Config: Pieces not found${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ MCP Config: File not found${NC}"
    fi
}

# Main installation flow
main() {
    echo "Starting Pieces MCP setup for BTPC project..."
    echo ""

    # Check Pieces OS
    if ! check_pieces_os; then
        echo ""
        echo -e "${YELLOW}Pieces OS not found.${NC}"
        echo "Please download and install Pieces OS from:"
        echo "https://pieces.app/install"
        echo ""
        read -p "Have you installed Pieces OS? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Please install Pieces OS and run this script again."
            exit 1
        fi
    fi

    # Start Pieces OS if not running
    if ! check_pieces_running; then
        echo ""
        read -p "Start Pieces OS now? (Y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            start_pieces_os
        else
            echo -e "${YELLOW}Please start Pieces OS manually${NC}"
            exit 1
        fi
    fi

    # Install Pieces CLI
    echo ""
    read -p "Install Pieces CLI? (Y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        install_pieces_cli
    fi

    # Run MCP setup
    echo ""
    read -p "Configure Pieces MCP for Claude? (Y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        run_pieces_mcp_setup
    fi

    # Add permissions
    echo ""
    add_permissions

    # Verify
    echo ""
    verify_installation

    # Final instructions
    echo ""
    echo "======================================"
    echo "✅ Pieces MCP Setup Complete!"
    echo "======================================"
    echo ""
    echo "Next steps:"
    echo "1. Restart Claude Code"
    echo "2. Verify Pieces tools are available"
    echo "3. Try: 'Save this code snippet to Pieces'"
    echo ""
    echo "Documentation: docs/MCP_INTEGRATION_GUIDE.md"
    echo ""
}

# Run main function
main