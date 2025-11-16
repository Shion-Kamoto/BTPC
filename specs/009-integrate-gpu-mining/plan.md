# Implementation Plan: GPU Mining Phase 3 - Stats & Monitoring

**Branch**: `009-integrate-gpu-mining` | **Date**: 2025-11-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/spec.md`

## Summary

Phase 3 adds GPU statistics collection and desktop app display for real-time mining monitoring. Extends existing GPU miner (Phase 2) with comprehensive device info, hashrate tracking, and HTTP stats endpoint. Desktop app displays GPU performance alongside CPU mining stats.

**Current State** (Phase 2 Complete):
- ✅ GPU mining functional with standalone OpenCL kernel
- ✅ GpuMiner struct with device detection
- ✅ SHA-512 mining on GPU (100-500 MH/s expected)

**Phase 3 Scope**:
- GPU stats data structure (device name, compute units, memory, hashrate)
- HTTP stats server (`http://127.0.0.1:18360/stats`)
- Desktop app stats display UI
- Real-time performance monitoring

## Technical Context

**Language/Version**: Rust 1.75+ (btpc_miner), JavaScript ES6+ (desktop app frontend)
**Primary Dependencies**:
- `warp` 0.3 (HTTP server for stats endpoint)
- `ocl` 0.19 (OpenCL - already integrated in Phase 2)
- `serde/serde_json` (stats serialization)
- Tauri 2.0 (desktop app IPC)

**Storage**: In-memory stats (no persistence required)
**Testing**: Unit tests for `get_stats()`, integration tests for HTTP endpoint
**Target Platform**: Linux/Windows desktop (OpenCL compatible)
**Project Type**: Desktop app (Tauri backend + vanilla JS frontend)

**Performance Goals**:
- Stats retrieval < 1ms
- HTTP endpoint < 10ms response time
- UI update rate: 1-2 seconds

**Constraints**:
- OpenCL DeviceInfo API limitations (no standard temp/power APIs)
- Thread-safe stats access (mining thread + stats server)
- No RPC complexity (simple HTTP GET endpoint)

**Scale/Scope**: Single GPU per miner instance (multi-GPU deferred)

## Constitution Check
*GATE: Must pass before Phase 0 research*

✅ **Security Gate**: No cryptographic operations in stats layer - PASS
✅ **Testing Gate**: Unit tests for `get_stats()`, integration tests for endpoint - PLANNED
✅ **Performance Gate**: Stats retrieval <1ms, minimal mining overhead - DESIGN TARGET
✅ **Memory Safety Gate**: All Rust, no unsafe blocks required - PASS
✅ **Dependency Gate**: `warp` widely used, audited HTTP library - ACCEPTABLE
✅ **Desktop App Gate** (Article XI): Backend-first validation, event-driven updates - COMPLIANT

**Initial Constitution Check**: ✅ PASS

## Project Structure

### Documentation (this feature)
```
specs/009-integrate-gpu-mining/
├── spec.md              # Feature specification (existing)
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (COMPLETE)
├── data-model.md        # Phase 1 output (COMPLETE)
├── quickstart.md        # Phase 1 output (COMPLETE)
├── contracts/           # Phase 1 output (COMPLETE)
│   └── gpu-stats-api.yaml
└── tasks.md             # Phase 2 output (/tasks command - pending)
```

### Source Code (repository root)
```
bins/btpc_miner/
├── src/
│   ├── main.rs                      # [MODIFY] Start stats server
│   ├── gpu_miner.rs                 # [MODIFY] Add get_stats() method
│   └── stats_server.rs              # [CREATE] HTTP stats endpoint
└── Cargo.toml                       # [MODIFY] Add warp dependency

btpc-desktop-app/
├── src-tauri/src/
│   ├── main.rs                      # [MODIFY] Register stats command
│   └── gpu_stats_commands.rs       # [CREATE] Tauri GPU stats command
└── ui/
    ├── mining.html                  # [MODIFY] Add GPU stats display
    └── js/
        └── mining-stats.js          # [CREATE] GPU stats UI logic

tests/
└── integration/
    └── test_gpu_stats_endpoint.rs   # [CREATE] Stats API integration test
```

**Structure Decision**: Desktop wallet app (Tauri) with HTTP stats bridge. Miner runs stats server, desktop app polls via HTTP (simpler than complex IPC). Constitution Article XI compliant - backend serves stats, frontend displays.

## Phase 0: Outline & Research

**Research Output**: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/research.md` ✅ COMPLETE

**Key Findings** (from research.md):
1. **OpenCL DeviceInfo API**: Standard API provides device name, vendor, compute units, memory, clock frequency (no vendor extensions needed)
2. **Thread Safety**: Arc<GpuMiner> with immutable get_stats() queries (read-only, naturally thread-safe)
3. **HTTP Server**: Warp 0.3 chosen (lightweight, tokio-compatible, simple filters)
4. **Tauri Integration**: Backend proxy approach (reqwest in Tauri command, Article XI compliant)

**Alternatives Rejected**:
- JSON-RPC: Too heavy for simple stats query
- WebSocket: Polling 1-2 seconds sufficient
- Vendor-specific monitoring: Deferred (requires NVML/ADL)

## Phase 1: Design & Contracts

### Data Model
**Output**: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/data-model.md` ✅ COMPLETE

**Entity: GpuStats**
```rust
pub struct GpuStats {
    pub device_name: String,       // OpenCL device name
    pub vendor: String,             // GPU manufacturer
    pub compute_units: u32,         // CUs/SMs
    pub max_work_group_size: usize, // Max threads per workgroup
    pub global_mem_size: u64,       // VRAM in bytes
    pub local_mem_size: u64,        // Local mem in bytes
    pub max_clock_frequency: u32,   // MHz
    pub hashrate: f64,              // MH/s (calculated)
    pub total_hashes: u64,          // Lifetime hashes
    pub uptime_seconds: u64,        // Mining uptime
    pub temperature: Option<f32>,   // °C (optional)
    pub power_usage: Option<f32>,   // W (optional)
}
```

**Validation Rules**:
- `device_name`: Non-empty OpenCL device string
- `hashrate`: Calculated as `total_hashes / uptime_seconds / 1_000_000.0`
- `temperature/power_usage`: Optional (not available via standard OpenCL)

### API Contracts
**Output**: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/contracts/gpu-stats-api.yaml` ✅ COMPLETE

**Endpoints**:
- `GET /stats`: Returns GpuStats JSON (200 OK or 500 error)
- `GET /health`: Returns `{"status": "ok", "service": "btpc_miner_stats"}`

**OpenAPI 3.0 Specification**: See contracts/gpu-stats-api.yaml

### Quickstart
**Output**: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/quickstart.md` ✅ COMPLETE

**Manual Test Scenario**:
```bash
# Terminal 1: Start GPU miner with stats server
/home/bob/.btpc/bin/btpc_miner --gpu --network regtest --address test_address

# Terminal 2: Query stats endpoint
curl http://127.0.0.1:18360/stats | jq

# Terminal 3: Launch desktop app
cd btpc-desktop-app && npm run tauri:dev
# Verify GPU stats panel displays
```

**Success Criteria**:
- Stats server starts without errors
- `/stats` endpoint returns valid JSON
- Desktop app displays GPU stats in real-time
- Stats query latency < 10ms, mining overhead < 1%

## Post-Design Constitution Check

Re-evaluating after Phase 1 design:

✅ **Security Gate**: Stats are read-only, no authentication needed (local-only endpoint) - PASS
✅ **Testing Gate**: Contract tests for /stats, unit tests for get_stats(), quickstart manual test - PASS
✅ **Performance Gate**: Stats retrieval uses OpenCL queries (<1ms), HTTP adds <10ms - ACCEPTABLE
✅ **Memory Safety Gate**: Warp is safe Rust, no new unsafe blocks - PASS
✅ **Desktop App Gate**: Frontend polls backend stats, displays results (Article XI compliant) - PASS

**Post-Design Constitution Check**: ✅ PASS

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
1. Contract test tasks (TDD):
   - `tests/integration/test_gpu_stats_endpoint.rs` - HTTP endpoint contract test [P]
   - `bins/btpc_miner/src/gpu_miner.rs` tests - get_stats() unit test [P]

2. Implementation tasks (make tests pass):
   - Create `bins/btpc_miner/src/stats_server.rs` - HTTP server module
   - Modify `bins/btpc_miner/src/main.rs` - Start stats server on miner launch
   - Create `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs` - Tauri command
   - Modify `btpc-desktop-app/ui/mining.html` - GPU stats display section
   - Create `btpc-desktop-app/ui/js/mining-stats.js` - Stats polling logic

3. Integration tasks:
   - Update `btpc-desktop-app/src-tauri/src/main.rs` - Register GPU stats command
   - Rebuild `btpc_miner` with warp dependency
   - Manual test quickstart scenario
   - Performance validation (stats overhead < 1%)

**Ordering Strategy**:
1. [P] Write contract tests (parallel - independent)
2. Implement stats server (sequential - depends on contracts)
3. Implement desktop app stats display (sequential - depends on server)
4. Integration and manual testing (final validation)

**Estimated Output**: 12-15 numbered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md, TDD approach)
**Phase 5**: Validation (quickstart.md manual test, performance check)

## Complexity Tracking

No constitutional violations requiring justification.

## Progress Tracking

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning approach documented (/plan command)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved (none present)
- [x] Complexity deviations documented (none required)

---
*Based on Constitution v1.0.1 - See `/home/bob/BTPC/BTPC/.specify/memory/constitution.md`*