# Research & Technical Decisions: GPU Mining Dashboard

**Feature**: 012-create-an-new
**Date**: 2025-11-11
**Status**: Complete

## Research Questions Resolved

All ambiguities were resolved during the `/clarify` workflow. No NEEDS CLARIFICATION markers remain in the specification.

## Technical Decisions

### 1. GPU Health Monitoring Library

**Decision**: Use platform-specific libraries via Rust FFI bindings:
- **NVIDIA**: NVML (NVIDIA Management Library) via `nvml-wrapper` crate
- **AMD**: ADL (AMD Display Library) via custom FFI bindings or `opencl3` device info
- **Fallback**: OpenCL device queries for basic info when vendor libraries unavailable

**Rationale**:
- NVML provides comprehensive NVIDIA GPU metrics (temperature, fan speed, power, memory, clock speed)
- ADL provides equivalent functionality for AMD GPUs
- OpenCL provides cross-vendor GPU enumeration and basic device info
- Rust FFI bindings provide memory-safe access to C libraries

**Alternatives Considered**:
- ❌ **Pure OpenCL**: Insufficient - OpenCL doesn't expose temperature/fan/power metrics
- ❌ **Sysfs/proc filesystem**: Linux-only, unreliable across distros, no Windows/macOS support
- ❌ **lm-sensors**: Requires external dependency, inconsistent GPU support

### 2. Event System for Real-Time Updates

**Decision**: Tauri event system with 5-second polling interval

**Rationale**:
- Article XI Section 11.3 mandates event-driven architecture (no frontend polling)
- Backend polls GPU sensors every 5 seconds (user-clarified requirement)
- Backend emits `"gpu-stats-updated"` event with all GPU data
- Frontend subscribes to events and updates UI reactively
- 5-second interval balances responsiveness with overhead (<1% mining impact)

**Alternatives Considered**:
- ❌ **Frontend polling**: Violates Article XI Section 11.7 (prohibited pattern)
- ❌ **1-second interval**: Higher overhead, may impact mining hashrate
- ❌ **WebSocket**: Unnecessary complexity for local Tauri app

### 3. Persistent Stats Storage

**Decision**: JSON file at `~/.btpc/data/mining_stats_per_gpu.json`

**Rationale**:
- Simple format for per-GPU lifetime blocks found
- Atomic writes prevent corruption
- Portable across platforms (Windows/Linux/macOS)
- No need for RocksDB complexity (small dataset, infrequent writes)

**Schema**:
```json
{
  "gpu_0": { "blocks_found": 42, "last_updated": "2025-11-11T10:00:00Z" },
  "gpu_1": { "blocks_found": 37, "last_updated": "2025-11-11T10:00:00Z" }
}
```

**Alternatives Considered**:
- ❌ **RocksDB**: Overkill for small dataset, adds storage overhead
- ❌ **SQLite**: Unnecessary complexity for simple key-value storage
- ❌ **Memory-only**: Would lose lifetime stats on app restart (FR-019 violation)

### 4. Thermal Throttling Algorithm

**Decision**: Incremental reduction with hysteresis

**Algorithm**:
1. Monitor GPU temperature every 5 seconds
2. If `temp > threshold` (default 80°C): Reduce mining intensity by 10%
3. Wait 10 seconds, re-check temperature
4. Repeat reduction until `temp < threshold`
5. If `temp < (threshold - 5°C)`: Restore full mining intensity incrementally

**Rationale**:
- Incremental approach prevents sudden hashrate drops
- Hysteresis (5°C gap) prevents oscillation
- Per-GPU throttling allows mixed-workload scenarios
- User-clarified requirement (Question 5: show warning + auto-throttle)

**Alternatives Considered**:
- ❌ **Immediate shutdown**: Too aggressive, stops all mining
- ❌ **Fixed throttle (50%)**: Not adaptive to cooling capacity
- ❌ **User manual control**: Requires constant monitoring (not practical)

### 5. Temperature Threshold Configuration

**Decision**: Configurable via Settings page, stored in localStorage, default 80°C

**Rationale**:
- User-clarified requirement (Question 3: configurable with 80°C default)
- Range: 60°C - 95°C (prevents unsafe extremes)
- Persisted in localStorage for UI convenience
- Backend validates threshold before applying (Article XI Section 11.2)

**Flow**:
1. User changes threshold in Settings → Frontend sends to backend
2. Backend validates range (60-95°C)
3. If valid: Backend saves, emits event, frontend updates localStorage
4. If invalid: Backend returns error, frontend shows error, NO localStorage save

**Alternatives Considered**:
- ❌ **Fixed threshold**: Not flexible for different GPU models/cooling setups
- ❌ **Auto-detect per-GPU**: Complex, unreliable, vendor-specific limits vary

### 6. Efficiency Metrics Calculation

**Decision**: Calculate both energy efficiency (H/W) and thermal efficiency (H/°C)

**Formulas**:
- Energy efficiency = `hashrate / power_consumption` (H/W or KH/W)
- Thermal efficiency = `hashrate / temperature` (H/°C or KH/°C)

**Edge Cases**:
- If `power_consumption == 0` or sensor unavailable → Display "N/A"
- If `temperature == 0` or sensor unavailable → Display "N/A"

**Rationale**:
- User-clarified requirement (Question 4: both metrics for comprehensive analysis)
- Energy efficiency helps optimize profitability (electricity cost)
- Thermal efficiency indicates cooling effectiveness
- Graceful degradation for missing sensors (FR-013, FR-026)

**Alternatives Considered**:
- ❌ **Hashrate/watt only**: Doesn't indicate thermal health
- ❌ **Performance score**: Abstract metric, less actionable than raw ratios

## Performance Validation

**Measured Requirements**:
- GPU enumeration: Target <500ms (NFR-001)
- Stats update overhead: Target <1% mining impact (NFR-002)
- UI render time: Target <200ms for 10+ GPUs (NFR-003)
- Sensor polling overhead: Negligible with 5-second interval (NFR-004)

**Benchmarking Plan**:
- Use `std::time::Instant` for timing measurements
- Test with 1, 4, 8, 16 GPU configurations
- Measure before/after hashrate with stats enabled
- Profile UI render time with Chrome DevTools Performance tab

## Dependencies Audit

**New Dependencies**:
- `nvml-wrapper = "0.9"` - NVIDIA GPU monitoring (audit required)
- `opencl3 = "0.9"` - Already in use (Feature 010), no new audit needed

**Security Considerations**:
- NVML/ADL libraries: Native vendor libraries, signed by GPU vendors
- Rust FFI bindings: Use safe wrappers, no `unsafe` blocks in user code
- Sensor access: Read-only operations, no write access to GPU firmware

## Article XI Compliance Summary

**Verified Patterns**:
- ✅ Backend (MiningThreadPool) is single source of truth
- ✅ Frontend displays only, never authoritative
- ✅ Backend emits "gpu-stats-updated" events
- ✅ Frontend listens for events (no polling)
- ✅ Event listeners cleaned up on beforeunload
- ✅ Temperature threshold: backend validation before localStorage save
- ✅ No duplicate notifications

**Constitutional Compliance**: PASS

---

**Research Complete**: 2025-11-11
**Next Phase**: Design & Contracts (data-model.md, contracts/, quickstart.md)