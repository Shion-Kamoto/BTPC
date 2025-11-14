# BTPC MCP Integration Guide

**Version:** 1.0
**Last Updated:** 2025-10-11
**Status:** Production Ready

---

## Table of Contents

1. [Overview](#overview)
2. [Currently Configured MCP Servers](#currently-configured-mcp-servers)
3. [Pieces MCP Setup](#pieces-mcp-setup)
4. [MCP Server Configuration](#mcp-server-configuration)
5. [BTPC-Specific MCP Usage](#btpc-specific-mcp-usage)
6. [Development Workflow Integration](#development-workflow-integration)
7. [Troubleshooting](#troubleshooting)
8. [Advanced Configuration](#advanced-configuration)

---

## Overview

Model Context Protocol (MCP) servers extend Claude Code with specialized capabilities. This guide covers:
- Setting up MCP servers for BTPC development
- Configuring Pieces MCP for code snippet management
- Using MCP tools in BTPC workflows
- Integrating with existing documentation and testing tools

### What is MCP?

MCP (Model Context Protocol) is an open protocol that allows Claude Code to connect with external tools and services. MCP servers provide:
- **Documentation search** (ref-tools-mcp)
- **Browser automation** (playwright-mcp)
- **Code snippet management** (pieces-mcp)
- **Custom integrations** (BTPC-specific tools)

---

## Currently Configured MCP Servers

The BTPC project has the following MCP servers pre-configured:

### 1. Ref Tools MCP (Documentation Search)

**Status:** ✅ Active
**Location:** `ref-tools-mcp/`
**Purpose:** Search Rust, Tauri, React, and blockchain documentation

**Available Tools:**
- `mcp__ref__ref_search_documentation` - Search public and private documentation
- `mcp__ref__ref_read_url` - Read documentation page content as markdown

**Usage Example:**
```
Search Tauri event system documentation
Read https://tauri.app/v1/guides/features/events
```

**Configured Permissions:**
```json
"mcp__ref__ref_search_documentation",
"mcp__ref__ref_read_url"
```

### 2. Playwright MCP (Browser Automation)

**Status:** ✅ Active
**Purpose:** Automated testing and UI interaction for BTPC desktop app

**Available Tools:**
- `mcp__playwright__browser_snapshot` - Capture accessibility snapshot
- `mcp__playwright__browser_navigate` - Navigate to URL
- `mcp__playwright__browser_take_screenshot` - Take screenshot
- `mcp__playwright__browser_click` - Perform click actions
- `mcp__playwright__browser_type` - Type text into elements

**Usage Example:**
```
Navigate to localhost:1420 (Tauri dev server)
Take screenshot of dashboard
Click wallet button
```

**Configured Permissions:**
```json
"mcp__playwright__browser_snapshot",
"mcp__playwright__browser_navigate",
"mcp__playwright__browser_take_screenshot"
```

---

## Pieces MCP Setup

Pieces MCP provides AI-powered code snippet management and context-aware suggestions.

### Prerequisites

1. **Install Pieces OS** (Required)
   ```bash
   # Linux (AppImage)
   wget https://builds.pieces.app/stages/production/pieces_os/linux-latest/pieces-os.AppImage
   chmod +x pieces-os.AppImage
   ./pieces-os.AppImage
   ```

2. **Install Pieces CLI** (Optional but recommended)
   ```bash
   # Using npm
   npm install -g @pieces.app/cli

   # Verify installation
   pieces --version
   ```

3. **Start Pieces OS**
   ```bash
   # Start Pieces OS (runs in background)
   ./pieces-os.AppImage &

   # Verify Pieces OS is running
   curl http://localhost:1000/health
   ```

### Configure Pieces MCP for Claude Code

#### Option 1: Using Pieces CLI (Recommended)

```bash
# In BTPC project root
cd /home/bob/BTPC/BTPC

# Run Pieces MCP setup for Claude
pieces mcp setup --claude

# This will:
# 1. Install pieces-mcp server
# 2. Configure Claude Code to use Pieces MCP
# 3. Add Pieces tools to available MCP tools
```

#### Option 2: Manual Configuration

1. **Install Pieces MCP server:**
   ```bash
   npm install -g @pieces.app/mcp
   ```

2. **Create MCP configuration file:**

   Create or edit `~/.config/claude/claude_desktop_config.json`:

   ```json
   {
     "mcpServers": {
       "pieces": {
         "command": "pieces-mcp",
         "env": {
           "PIECES_OS_URL": "http://localhost:1000"
         }
       }
     }
   }
   ```

3. **Restart Claude Code:**
   ```bash
   # Close Claude Code completely
   # Reopen Claude Code
   # Verify Pieces MCP tools are available
   ```

### Verify Pieces MCP Installation

```bash
# Check Pieces OS is running
curl http://localhost:1000/health
# Should return: {"status":"ok"}

# Check Pieces MCP is installed
npm list -g @pieces.app/mcp
# Should show installed version

# In Claude Code, check available MCP tools
# Look for mcp__pieces__* tools in tool list
```

### Pieces MCP Tools

Once configured, the following Pieces tools are available:

**Code Management:**
- `mcp__pieces__save_snippet` - Save code snippet to Pieces
- `mcp__pieces__search_snippets` - Search saved snippets
- `mcp__pieces__get_snippet` - Retrieve specific snippet
- `mcp__pieces__update_snippet` - Update existing snippet

**Context & Suggestions:**
- `mcp__pieces__get_context` - Get AI context for code
- `mcp__pieces__ask_copilot` - Ask Pieces Copilot questions
- `mcp__pieces__suggest_improvements` - Get code improvement suggestions

---

## MCP Server Configuration

### Configuration File Locations

**Claude Code Settings:**
- Local settings: `.claude/settings.local.json` (project-specific)
- Global settings: `~/.config/claude/claude_desktop_config.json` (all projects)

**BTPC Project Settings:**
```
/home/bob/BTPC/BTPC/.claude/settings.local.json
```

### Adding New MCP Server Permissions

To use a new MCP tool, add it to `.claude/settings.local.json`:

```json
{
  "permissions": {
    "allow": [
      // ... existing permissions ...
      "mcp__pieces__save_snippet",
      "mcp__pieces__search_snippets",
      "mcp__pieces__get_context"
    ]
  }
}
```

### MCP Server Environment Variables

**Ref Tools MCP:**
```bash
# No environment variables needed (uses public APIs)
```

**Playwright MCP:**
```bash
# Playwright browser configuration
export PLAYWRIGHT_BROWSER=chromium  # or firefox, webkit
export PLAYWRIGHT_HEADLESS=true     # or false for visible browser
```

**Pieces MCP:**
```bash
# Pieces OS connection
export PIECES_OS_URL=http://localhost:1000
export PIECES_API_KEY=<your-api-key>  # if using cloud features
```

---

## BTPC-Specific MCP Usage

### Use Case 1: Searching Rust/Tauri Documentation

**Scenario:** Need to understand Tauri event emission

```
User: How do I emit events from Tauri backend?

Claude uses: mcp__ref__ref_search_documentation
Query: "Tauri event emit rust backend"

Result: Finds relevant Tauri documentation
Claude reads: mcp__ref__ref_read_url
URL: https://tauri.app/v1/guides/features/events/

Provides: Code example for Arc<AppHandle> emit
```

### Use Case 2: Testing Desktop App with Playwright

**Scenario:** Automated testing of wallet creation flow

```
User: Test the wallet creation flow in the desktop app

Claude uses: mcp__playwright__browser_navigate
URL: http://localhost:1420

Claude uses: mcp__playwright__browser_snapshot
Analyzes: Available UI elements

Claude uses: mcp__playwright__browser_click
Element: "Create Wallet" button

Claude uses: mcp__playwright__browser_type
Element: Wallet name field
Text: "Test Wallet"

Claude uses: mcp__playwright__browser_take_screenshot
Captures: Wallet creation confirmation
```

### Use Case 3: Saving Reusable Code Snippets

**Scenario:** Save ML-DSA signature pattern for reuse

```
User: Save this ML-DSA signature code for future reference

Claude uses: mcp__pieces__save_snippet
Code: ML-DSA signature generation pattern
Tags: ["rust", "ml-dsa", "crypto", "btpc"]
Description: "Quantum-resistant signature generation"

Later retrieval:
Claude uses: mcp__pieces__search_snippets
Query: "ML-DSA signature"
Results: Previously saved snippet with context
```

### Use Case 4: Article XI Compliance Checking

**Scenario:** Verify desktop app follows Article XI patterns

```
User: Check if settings.html follows Article XI patterns

Claude uses: Read(settings.html)
Claude uses: mcp__ref__ref_search_documentation
Query: "Article XI desktop application patterns BTPC"

Claude analyzes:
- Backend-first validation ✅
- Event listener cleanup ✅
- No localStorage before backend ✅

Result: Constitutional compliance verified
```

---

## Development Workflow Integration

### Daily Development Workflow

**Morning Session (Resume Work):**
1. Run `/start` command
2. Claude reads constitution (Article XI)
3. Claude uses `mcp__ref__ref_search_documentation` for any unclear patterns
4. Begin implementation

**During Implementation:**
1. Need blockchain API reference?
   - Claude uses `mcp__ref__ref_search_documentation("Rust RocksDB API")`
2. Need to save reusable pattern?
   - Claude uses `mcp__pieces__save_snippet`
3. Need to test UI change?
   - Claude uses `mcp__playwright__browser_*` tools

**End of Session:**
1. Run `/stop` command
2. Claude documents session work
3. Claude may save important patterns to Pieces
4. Session handoff complete

### Debugging Workflow

**Problem:** Desktop app state not synchronizing

**Workflow:**
1. **Read Documentation:**
   ```
   Use mcp__ref__ref_search_documentation
   Query: "Tauri event listener not firing"
   ```

2. **Check Code Patterns:**
   ```
   Use mcp__pieces__search_snippets
   Query: "event listener setup btpc"
   ```

3. **Test Live App:**
   ```
   Use mcp__playwright__browser_navigate
   Use mcp__playwright__browser_snapshot
   Use mcp__playwright__browser_take_screenshot
   ```

4. **Find Solution:**
   ```
   Use mcp__ref__ref_read_url
   Read Tauri troubleshooting guide
   ```

5. **Save Fix:**
   ```
   Use mcp__pieces__save_snippet
   Save working event listener pattern
   ```

### Testing Workflow

**Manual Testing Guide Integration:**

The `MANUAL_TESTING_GUIDE.md` can be enhanced with MCP automation:

```
# Automated Article XI Compliance Testing

## Network Config Synchronization (Article XI, Section 11.3)

### Manual Test:
1. Change network on settings page
2. Navigate to dashboard
3. Verify footer shows new network

### MCP-Automated Test:
1. mcp__playwright__browser_navigate (settings)
2. mcp__playwright__browser_click (network dropdown)
3. mcp__playwright__browser_click (testnet option)
4. mcp__playwright__browser_navigate (dashboard)
5. mcp__playwright__browser_snapshot (verify state)
6. mcp__playwright__browser_take_screenshot (evidence)
```

---

## Troubleshooting

### Common Issues

#### 1. Pieces OS Not Running

**Symptom:** `mcp__pieces__*` tools unavailable or timeout

**Solution:**
```bash
# Check Pieces OS status
curl http://localhost:1000/health

# If not running, start Pieces OS
./pieces-os.AppImage &

# Restart Claude Code
```

#### 2. Playwright Browser Not Found

**Symptom:** `mcp__playwright__*` tools fail with browser error

**Solution:**
```bash
# Install Playwright browsers
npx playwright install chromium

# Or install all browsers
npx playwright install
```

#### 3. Ref Tools Documentation Not Found

**Symptom:** `mcp__ref__ref_search_documentation` returns no results

**Solution:**
```bash
# Update ref-tools-mcp
cd ref-tools-mcp/
npm update
npm run build

# Restart Claude Code
```

#### 4. MCP Server Connection Timeout

**Symptom:** MCP tool calls timeout or hang

**Solution:**
```bash
# Check MCP server processes
ps aux | grep mcp

# Kill stale processes
pkill -f "pieces-mcp"
pkill -f "ref-tools"

# Restart Claude Code
```

#### 5. Permission Denied Errors

**Symptom:** MCP tool blocked by permissions

**Solution:**
```json
// Add to .claude/settings.local.json
{
  "permissions": {
    "allow": [
      "mcp__toolname__*"  // Add wildcard for tool family
    ]
  }
}
```

### Debug Mode

Enable MCP debug logging:

```bash
# Set environment variable before starting Claude Code
export MCP_DEBUG=1

# Or in Claude Code settings
{
  "mcp": {
    "debug": true,
    "logLevel": "verbose"
  }
}
```

---

## Advanced Configuration

### Custom MCP Server for BTPC

Create a BTPC-specific MCP server for blockchain operations:

**1. Create server directory:**
```bash
mkdir -p mcp/btpc-mcp-server
cd mcp/btpc-mcp-server
npm init -y
```

**2. Install MCP SDK:**
```bash
npm install @modelcontextprotocol/sdk
```

**3. Create server implementation:**
```typescript
// index.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';

// BTPC blockchain tools
const tools = [
  {
    name: 'get_blockchain_info',
    description: 'Get BTPC blockchain information',
    inputSchema: {
      type: 'object',
      properties: {
        rpc_port: {
          type: 'number',
          description: 'RPC port (default: 18360)',
        },
      },
    },
  },
  {
    name: 'validate_ml_dsa_signature',
    description: 'Validate ML-DSA (Dilithium5) signature',
    inputSchema: {
      type: 'object',
      properties: {
        public_key: { type: 'string' },
        signature: { type: 'string' },
        message: { type: 'string' },
      },
      required: ['public_key', 'signature', 'message'],
    },
  },
];

// Server setup
const server = new Server(
  {
    name: 'btpc-mcp-server',
    version: '1.0.0',
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// List tools handler
server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools,
}));

// Call tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  if (name === 'get_blockchain_info') {
    const rpcPort = args?.rpc_port || 18360;
    const result = await fetch(`http://127.0.0.1:${rpcPort}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: '1',
        method: 'getblockchaininfo',
        params: [],
      }),
    });
    const data = await result.json();
    return { content: [{ type: 'text', text: JSON.stringify(data, null, 2) }] };
  }

  if (name === 'validate_ml_dsa_signature') {
    // Call Rust validation function
    // (Implementation depends on btpc-core API)
    return { content: [{ type: 'text', text: 'Signature validation result' }] };
  }

  throw new Error(`Unknown tool: ${name}`);
});

// Start server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch(console.error);
```

**4. Build and configure:**
```bash
# Build server
npm run build

# Add to Claude config
{
  "mcpServers": {
    "btpc": {
      "command": "node",
      "args": ["/home/bob/BTPC/BTPC/mcp/btpc-mcp-server/dist/index.js"]
    }
  }
}
```

### MCP Server Testing

Test MCP servers before integrating:

```bash
# Test Pieces MCP connection
curl http://localhost:1000/api/discover

# Test ref-tools-mcp
cd ref-tools-mcp/
npm test

# Test custom BTPC MCP server
node mcp/btpc-mcp-server/dist/index.js --test
```

### Performance Optimization

**MCP Server Caching:**
```typescript
// Cache documentation searches
const cache = new Map();

async function searchDocs(query: string) {
  if (cache.has(query)) {
    return cache.get(query);
  }
  const result = await performSearch(query);
  cache.set(query, result);
  return result;
}
```

**Playwright Browser Reuse:**
```javascript
// Keep browser open between tests
const browser = await chromium.launch({ headless: true });
// Reuse browser for multiple operations
const page = await browser.newPage();
```

---

## MCP Integration Checklist

### Initial Setup
- [ ] Install Pieces OS
- [ ] Run `pieces mcp setup --claude`
- [ ] Verify Pieces MCP tools available
- [ ] Add Pieces tools to `.claude/settings.local.json` permissions
- [ ] Test Pieces snippet save/retrieve

### Ref Tools MCP
- [x] Configured (already active)
- [x] Documentation search working
- [x] URL reading functional
- [ ] Custom BTPC documentation indexed

### Playwright MCP
- [x] Configured (already active)
- [x] Browser automation working
- [x] Screenshots functional
- [ ] Desktop app testing automated

### Custom BTPC MCP Server (Optional)
- [ ] Server implementation created
- [ ] Blockchain RPC integration
- [ ] ML-DSA validation tools
- [ ] Wallet operation tools
- [ ] Testing complete
- [ ] Documented in this guide

---

## Resources

### Documentation
- **MCP Protocol**: https://modelcontextprotocol.io/
- **Pieces MCP**: https://docs.pieces.app/extensions-plugins/mcp
- **Ref Tools**: https://github.com/kodu-ai/ref-tools-mcp
- **Playwright MCP**: https://github.com/microsoft/playwright-mcp

### BTPC Documentation
- **Project Guidelines**: `/home/bob/BTPC/BTPC/CLAUDE.md`
- **Constitution**: `.specify/memory/constitution.md`
- **Manual Testing**: `MANUAL_TESTING_GUIDE.md`
- **Status**: `STATUS.md`

### Support
- **BTPC Issues**: https://github.com/btpc-project/issues
- **MCP Community**: https://discord.gg/modelcontextprotocol
- **Pieces Support**: https://support.pieces.app/

---

## Next Steps

1. **Complete Pieces MCP Setup:**
   ```bash
   pieces mcp setup --claude
   ```

2. **Test Integration:**
   ```bash
   # In Claude Code, try:
   "Search Rust documentation for Arc<RwLock>"
   "Save this ML-DSA pattern to Pieces"
   ```

3. **Enhance BTPC Workflow:**
   - Add MCP automation to testing guide
   - Save reusable patterns to Pieces
   - Document blockchain-specific searches

4. **Create Custom MCP Server (Optional):**
   - Implement BTPC blockchain tools
   - Add ML-DSA validation helpers
   - Integrate with btpc-core library

---

**Status:** Documentation Complete
**Pieces MCP:** Ready for setup
**Next Action:** Run `pieces mcp setup --claude`

**Last Updated:** 2025-10-11 18:10 UTC
**Maintained By:** BTPC Development Team