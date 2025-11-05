# BTPC MCP Quick Start Guide

**5-Minute Setup** | Last Updated: 2025-10-11

---

## Quick Setup

### 1. Install Pieces OS (One-Time Setup)

```bash
# Download Pieces OS
wget https://builds.pieces.app/stages/production/pieces_os/linux-latest/pieces-os.AppImage

# Make executable
chmod +x pieces-os.AppImage

# Run Pieces OS
./pieces-os.AppImage &
```

### 2. Run Setup Script

```bash
# From BTPC project root
./scripts/setup-pieces-mcp.sh

# Follow prompts:
# - Confirms Pieces OS is running
# - Installs Pieces CLI
# - Configures Pieces MCP for Claude
# - Adds permissions
```

### 3. Restart Claude Code

Close and reopen Claude Code to load new MCP configuration.

---

## Quick Test

### Test 1: Search Documentation
```
User: "Search Rust documentation for Arc<RwLock>"

Claude will use: mcp__ref__ref_search_documentation
Result: Finds relevant Rust std library docs
```

### Test 2: Save Code Snippet
```
User: "Save this ML-DSA signature pattern to Pieces"

Claude will use: mcp__pieces__save_snippet
Result: Code saved with tags and description
```

### Test 3: Test Desktop App
```
User: "Navigate to localhost:1420 and take a screenshot"

Claude will use:
- mcp__playwright__browser_navigate
- mcp__playwright__browser_take_screenshot

Result: Screenshot of Tauri dev server
```

---

## Available MCP Tools

### Documentation (Ref Tools) - ✅ Active
- Search Rust, Tauri, React docs
- Read documentation pages
- **Usage:** "Search Tauri event documentation"

### Browser Automation (Playwright) - ✅ Active
- Navigate to URLs
- Click elements
- Take screenshots
- **Usage:** "Test the wallet creation flow"

### Code Snippets (Pieces) - ⏳ Pending Setup
- Save code snippets
- Search saved snippets
- Get AI context
- **Usage:** "Save this pattern to Pieces"

---

## Common Commands

### Search BTPC-Specific Documentation
```
"Search Tauri backend-first validation patterns"
"Find Rust Arc<RwLock> documentation"
"Look up ML-DSA signature examples"
```

### Save Reusable Patterns
```
"Save this Article XI validation pattern"
"Store this UTXO management code"
"Keep this ML-DSA signature helper"
```

### Automated Testing
```
"Test network switching in settings"
"Navigate to dashboard and verify balance"
"Screenshot the wallet manager page"
```

---

## Troubleshooting

### Pieces Not Working
```bash
# Check Pieces OS
curl http://localhost:1000/health

# Restart Pieces OS
pkill pieces-os
./pieces-os.AppImage &
```

### MCP Tools Not Available
```bash
# Restart Claude Code
# Check .claude/settings.local.json permissions
# Verify MCP configuration
```

### Permission Denied
Add to `.claude/settings.local.json`:
```json
"mcp__pieces__save_snippet",
"mcp__pieces__search_snippets"
```

---

## Full Documentation

**Complete Guide:** `docs/MCP_INTEGRATION_GUIDE.md`
- Detailed setup instructions
- Advanced configuration
- Custom MCP servers
- BTPC-specific workflows

---

## Next Steps

1. ✅ Setup complete? Try saving a code snippet
2. 🔍 Explore ref tools with documentation searches
3. 🤖 Test Playwright browser automation
4. 📚 Read full guide: `docs/MCP_INTEGRATION_GUIDE.md`

---

**Quick Help:**
- Setup issues? Run: `./scripts/setup-pieces-mcp.sh`
- Tool not working? Check: `.claude/settings.local.json`
- Need more info? Read: `docs/MCP_INTEGRATION_GUIDE.md`