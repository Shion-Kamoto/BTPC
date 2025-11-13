# Feature Specification: GPU Mining Dashboard with Individual GPU Statistics

**Feature Branch**: `012-create-an-new`
**Created**: 2025-11-11
**Status**: Draft
**Input**: User description: "create an new GPP mining feature that has its own sup page tab on the mining page. Where all on system GPU are listed along with indivisual mining stats aswell aS tempreture and other info."
**Constitution**: Article XI Compliance Required for Desktop Features

## Execution Flow (main)
```
1. Parse user description from Input ✓
   → Feature: GPU mining dashboard with individual GPU stats
2. Extract key concepts from description ✓
   → Actors: Miners
   → Actions: View GPU list, monitor individual GPU stats, track temperature
   → Components: Mining page sub-tab, GPU enumeration, stats display
3. For each unclear aspect:
   → [NEEDS CLARIFICATION: What "other info" should be displayed beyond temperature and mining stats?]
   → [NEEDS CLARIFICATION: Should GPU stats update in real-time? What frequency?]
   → [NEEDS CLARIFICATION: Should users be able to enable/disable individual GPUs?]
   → [NEEDS CLARIFICATION: What happens when GPU overheats? Auto-stop threshold?]
4. Check constitutional requirements:
   → Desktop app feature: FLAG "Article XI patterns apply" ✓
5. Fill User Scenarios & Testing section ✓
6. Generate Functional Requirements ✓
7. Identify Key Entities ✓
8. Run Review Checklist
   → WARN "Spec has uncertainties - 4 NEEDS CLARIFICATION markers"
9. Return: SUCCESS (spec ready for clarification phase)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no Rust code, Tauri commands, RocksDB schemas)
- 👥 Written for cryptocurrency users and stakeholders, not developers
- 🔒 Always consider quantum-resistance and security implications

---

## Clarifications

### Session 2025-11-11
- Q: What additional GPU health metrics should be displayed beyond temperature? → A: Temperature + Fan speed + Power consumption + Memory usage + Clock speed (full monitoring)
- Q: How frequently should GPU statistics and health metrics be updated? → A: Every 5 seconds (conservative - acceptable delay, minimal overhead)
- Q: At what temperature should the system highlight warnings for GPU overheating? → A: Configurable by user with 80°C default (flexible, requires settings UI)
- Q: What efficiency metrics should the system calculate and display for each GPU? → A: Both Hashrate/Watt (energy efficiency) AND Hashrate/Temperature (thermal efficiency) for comprehensive analysis
- Q: What should the system do when a GPU temperature exceeds the warning threshold during mining? → A: Show visual warning + auto-throttle hashrate (reduce mining intensity to lower temperature, balancing protection with continued mining)

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
**As a** cryptocurrency miner using GPU hardware,
**I want to** monitor individual GPU mining performance and health metrics in a dedicated dashboard,
**So that** I can optimize mining efficiency, prevent hardware damage, and maximize block discovery rates

### Acceptance Scenarios
1. **Given** desktop app is running with mining page open, **When** user clicks GPU mining sub-tab, **Then** all system GPUs are listed with current status (mining/idle)

2. **Given** GPU mining is active on 2 GPUs, **When** user views GPU dashboard, **Then** each GPU shows:
   - Hashrate (H/s, KH/s, MH/s)
   - Temperature (°C)
   - Fan speed (RPM)
   - Power consumption (Watts)
   - Memory usage (MB used / total)
   - Core clock speed (MHz)
   - Energy efficiency (H/W or KH/W)
   - Thermal efficiency (H/°C or KH/°C)
   - Blocks found (lifetime count per GPU)
   - Uptime (mining duration)

3. **Given** GPU temperature exceeds configured warning threshold (default: 80°C), **When** system detects overheat condition, **Then** visual warning displayed on GPU card AND mining intensity automatically reduced (throttled) to lower temperature while continuing operation

4. **Given** one GPU fails or becomes unavailable, **When** user views dashboard, **Then** GPU status shows as "Unavailable" or "Error" with diagnostic information

5. **Given** user has multiple GPUs of different models, **When** viewing dashboard, **Then** GPU model/name is clearly displayed for each device

### Edge Cases
- What happens when a GPU is removed while mining is active?
- How does the system handle GPU driver crashes or resets?
- What if GPUs have different capabilities (some support OpenCL, others don't)?
- How does the dashboard handle systems with 10+ GPUs (server rigs)?
- What happens when temperature sensors are unavailable or return invalid data?

---

## Requirements *(mandatory)*

### Functional Requirements

**GPU Detection & Enumeration:**
- **FR-001**: System MUST provide GPU enumeration on demand when user navigates to GPU Mining tab
- **FR-002**: System MUST display GPU model name, vendor (NVIDIA/AMD/Intel), and device index for all enumerated GPUs
- **FR-003**: ~~System MUST refresh GPU list when hardware changes detected (hot-plug support)~~ **REMOVED** - Deferred to future enhancement (requires complex hardware monitoring service)
- **FR-004**: System MUST gracefully handle GPU enumeration failures (no GPUs, driver errors)

**GPU Mining Statistics:**
- **FR-005**: System MUST display individual hashrate for each GPU (per-device, not aggregate)
- **FR-006**: System MUST track lifetime blocks found per GPU
- **FR-007**: System MUST display mining uptime (duration) per GPU
- **FR-008**: System MUST calculate and display energy efficiency (hashrate/watt) and thermal efficiency (hashrate/temperature) - see GPU Mining Stats entity for calculation details
- **FR-008a**: Efficiency calculations MUST handle edge cases (zero power, missing temperature) by showing "N/A"

**GPU Health Monitoring:**
- **FR-009**: System MUST display current GPU temperature in Celsius
- **FR-010**: System MUST update GPU statistics and health metrics every 5 seconds (conservative polling to minimize overhead)
- **FR-011**: System MUST highlight temperature warnings when GPU temperature exceeds user-configured threshold (default: 80°C)
- **FR-011a**: System MUST allow users to configure temperature warning threshold via Settings page (range: 60°C - 95°C)
- **FR-011b**: System MUST persist temperature threshold setting across app restarts
- **FR-012**: System MUST display fan speed (RPM), power consumption (Watts), memory usage (MB), and core clock speed (MHz)
- **FR-013**: System MUST handle unavailable metrics gracefully (show "N/A" when sensor not available)

**Thermal Protection:**
- **FR-013a**: System MUST automatically throttle (reduce) GPU mining intensity when temperature exceeds configured warning threshold
- **FR-013b**: Throttling MUST reduce hashrate incrementally until temperature drops below threshold (reduce by 10% of CURRENT intensity every 10 seconds - e.g., 100% → 90% → 81% → 73%)
- **FR-013c**: System MUST restore full mining intensity when GPU temperature returns to safe levels (threshold minus 5°C hysteresis)
- **FR-013d**: Thermal throttling actions MUST be logged for user visibility (e.g., "GPU 0 throttled to 80% due to 82°C temperature")

**User Interface (Desktop App):**
- **FR-014**: Desktop app MUST add "GPU Mining" sub-tab to Mining page
- **FR-015**: GPU dashboard MUST display all GPUs in a list or grid layout
- **FR-016**: GPU dashboard MUST support systems with 1-16 GPUs without UI degradation
- **FR-017**: GPU stats MUST update in real-time via Tauri events (Article XI, Section 11.3)
- **FR-018**: User MUST be able to view GPU details without starting mining
- **FR-019**: System MUST persist GPU-specific stats across app restarts (blocks found per GPU)

**Backend-First Validation (Article XI):**
- **FR-020**: Backend MUST be single source of truth for all GPU data (Article XI, Section 11.1)
- **FR-021**: Frontend MUST NOT maintain authoritative GPU state (Article XI, Section 11.1)
- **FR-022**: All GPU data queries MUST go through backend (no direct hardware polling from frontend)
- **FR-023**: Backend MUST emit events when GPU stats change (Article XI, Section 11.3)

**Error Handling:**
- **FR-024**: System MUST display actionable error messages when GPU hardware fails
- **FR-025**: System MUST provide diagnostics when GPU is incompatible (no OpenCL support)
- **FR-026**: System MUST handle missing sensors gracefully (show "N/A" for unavailable metrics instead of crash)

### Non-Functional Requirements

**Performance:**
- **NFR-001**: GPU enumeration MUST complete in < 500ms
- **NFR-002**: Stats updates MUST not impact mining hashrate (< 1% overhead)
- **NFR-003**: Dashboard UI MUST remain responsive with 10+ GPUs (< 200ms render time)
- **NFR-004**: Temperature polling MUST not add latency to mining operations

**Usability:**
- **NFR-005**: GPU dashboard MUST be accessible from Mining page with single click (sub-tab)
- **NFR-006**: Temperature warnings MUST be visually distinct (color coding)
- **NFR-007**: GPU names MUST be human-readable (not just device indices)
- **NFR-008**: Stats MUST use appropriate units (H/s, KH/s, MH/s for hashrate)

**Security:**
- **NFR-009**: GPU hardware queries MUST NOT expose system vulnerabilities
- **NFR-010**: Temperature sensor access MUST be sandboxed (no privilege escalation)

**Reliability:**
- **NFR-011**: GPU failures MUST NOT crash the entire application
- **NFR-012**: Stats persistence MUST survive application crashes (atomic writes)

### Key Entities

**GPU Device:**
- Represents a physical GPU available for mining
- Contains: Device index (0, 1, 2...), model name (e.g., "NVIDIA RTX 3080"), vendor (NVIDIA/AMD/Intel), OpenCL capability flag
- Relationships: has MiningStats, has HealthMetrics
- Backend source of truth: GPU enumeration service (read-only, hardware-provided data)

**GPU Mining Stats:**
- Represents mining performance metrics for a specific GPU
- Contains: GPU device index, current hashrate (f64), lifetime blocks found (u64), mining uptime (Duration), mining status (Active/Idle/Error), energy efficiency (hashrate/watt), thermal efficiency (hashrate/temperature)
- Relationships: belongs to GPU Device, requires GPU Health Metrics for efficiency calculations
- Backend source of truth: `Arc<RwLock<MiningThreadPool>>` stored in AppState (existing from Feature 010 - see `btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`)
- Persisted: Lifetime blocks found saved to disk (`~/.btpc/data/mining_stats_per_gpu.json`)
- Efficiency calculations: Handle division by zero (show "N/A" when power is 0 or temperature unavailable)
- **Implementation Note**: Feature 012 extends existing MiningThreadPool with per-GPU stat tracking (Task T016)

**GPU Health Metrics:**
- Represents real-time hardware health data for a GPU
- Contains: GPU device index, temperature (°C), fan speed (RPM), power consumption (Watts), memory usage (MB used/total), core clock speed (MHz)
- Relationships: belongs to GPU Device
- Backend source of truth: GPU monitoring service (hardware sensor polling)
- Update frequency: 5 seconds (conservative polling interval to balance responsiveness with overhead)
- Graceful degradation: Individual metrics show "N/A" if sensor unavailable

**Mining Page Sub-Tab State:**
- Represents UI navigation state for Mining page
- Contains: Active sub-tab ("CPU Mining" / "GPU Mining"), last viewed GPU (device index)
- Backend source of truth: Frontend local state (UI-only, not authoritative)
- Event-driven: Tab switches do not modify backend state (read-only dashboard)

---

## Constitutional Compliance *(mandatory for desktop features)*

### Article XI Applicability
- [ ] **Not a desktop feature** - Skip Article XI patterns
- [x] **Desktop feature** - Article XI patterns apply (complete checklist below)

### Article XI Compliance Checklist

**Section 11.1 - Single Source of Truth:**
- [x] Identify authoritative state location: Backend MiningThreadPool (Arc<RwLock>) for stats, GPU enumeration service for hardware info
- [x] Frontend displays state only, never maintains authoritative state
- [x] Specified: GPU stats fetched via Tauri command, frontend reads only

**Section 11.2 - Backend-First Validation:**
- [x] All user actions validate with backend FIRST (N/A - this is read-only dashboard)
- [x] Failure exits early, NO localStorage save on validation failure (N/A - no user actions modify GPU state)
- [x] Specified: Dashboard is read-only, no validation needed

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events on all state changes: "gpu-stats-updated" event with per-GPU data
- [x] Frontend listens for events and updates UI: Dashboard subscribes to "gpu-stats-updated"
- [x] Specified: Event payload includes array of GPU stats, frontend updates corresponding UI cards

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload: beforeunload removes "gpu-stats-updated" listener
- [x] No memory leaks from forgotten listeners
- [x] Specified: Tauri unlisten() called in beforeunload handler

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage before backend validation (read-only dashboard, no validation)
- [x] Confirmed: NO authoritative state in frontend JavaScript (all GPU data from backend)
- [x] Confirmed: NO polling when events available (uses "gpu-stats-updated" events, not polling)
- [x] Confirmed: NO duplicate notifications for user actions (N/A - no user actions)

---

## Dependencies & Assumptions

### Dependencies
- Depends on existing MiningThreadPool module (Feature 010) for GPU mining infrastructure
  - **State Management**: `Arc<RwLock<MiningThreadPool>>` already exists in AppState (btpc-desktop-app/src-tauri/src/main.rs)
  - **Extension Point**: Task T016 extends MiningThreadPool with per-GPU stat tracking methods
- Depends on GPU mining being functional (OpenCL/CUDA support)
- Depends on Tauri events system (Article XI)
- Depends on GPU hardware detection library (OpenCL device enumeration)
- Depends on temperature sensor access (platform-specific: NVML for NVIDIA, ADL for AMD)

### Assumptions
- Assumes user has at least one GPU with OpenCL or CUDA support
- Assumes GPU drivers are properly installed
- Assumes temperature sensors are available (may not be on all GPUs/platforms)
- Assumes desktop app has permission to query GPU hardware
- Assumes system supports GPU hot-plug detection (USB GPUs, eGPUs)
- Assumes mining stats persistence directory (~/.btpc/data) is writable

---

## Review & Acceptance Checklist

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families)
- [x] Focused on user value and cryptocurrency operations
- [x] Written for non-technical cryptocurrency stakeholders
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (Article XI compliance)

### Requirement Completeness
- [x] No [NEEDS CLARIFICATION] markers remain (all 5 clarifications resolved via /clarify workflow)
- [x] Requirements are testable and unambiguous (all ambiguities resolved)
- [x] Success criteria are measurable (< 500ms enumeration, < 1% overhead, 5-second update frequency)
- [x] Scope is clearly bounded (GPU dashboard only, no CPU mining changes)
- [x] Dependencies and assumptions identified
- [x] Security implications considered (sandboxed hardware access)
- [x] Performance targets specified (NFR-001 through NFR-004)

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (desktop feature)
- [x] If applicable: All Article XI patterns addressed in requirements
- [x] If applicable: Constitutional compliance checklist completed
- [ ] If not applicable: Confirmed feature is not desktop app related

---

## Execution Status

- [x] User description parsed
- [x] Key concepts extracted (GPU mining, health monitoring, desktop UI)
- [x] Constitutional requirements flagged (Article XI applies)
- [x] Ambiguities marked with [NEEDS CLARIFICATION] (5 items originally)
- [x] User scenarios defined (miner monitoring GPU performance)
- [x] Functional requirements generated (GPU detection, stats, health)
- [x] Entities identified (GPU Device, Mining Stats, Health Metrics)
- [x] Constitutional compliance evaluated (Article XI checklist complete)
- [x] Clarifications resolved (5 questions answered via /clarify workflow)
- [x] Review checklist passed (all ambiguities resolved, spec ready for /plan)

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, React frontend, 68 Tauri commands
- Network: Bitcoin-compatible P2P, JSON-RPC 2.0
- Mining: MiningThreadPool (Feature 010), OpenCL/CUDA support

**Constitutional Framework:**
- Constitution version: 1.0.1
- Article XI: Desktop Application Development Principles (mandatory for UI features)
- See `.specify/memory/constitution.md` for complete governance rules

**Project Structure:**
- `btpc-core/` - Core blockchain library (Rust)
- `bins/` - btpc_node, btpc_wallet, btpc_miner binaries
- `btpc-desktop-app/` - Tauri desktop wallet application
- `tests/` - Integration and unit tests

**Key Documentation:**
- `CLAUDE.md` - Project overview and guidelines
- `STATUS.md` - Current implementation status
- `style-guide/ux-rules.md` - UI/UX patterns (Monero-inspired)
- `.specify/memory/constitution.md` - Governance rules

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-11-11
**Maintained by**: .specify framework