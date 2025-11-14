# Data Model: GPU Mining Dashboard

**Feature**: 012-create-an-new
**Date**: 2025-11-11
**Status**: Complete

## Entity Relationship Diagram

```
GPU Device (1) ──< has >── (1) GPU Mining Stats
GPU Device (1) ──< has >── (1) GPU Health Metrics

Mining Page Sub-Tab State (UI-only, not authoritative)
```

## Core Entities

### 1. GPU Device

**Purpose**: Represents a physical GPU available for mining

**Fields**:
| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| `device_index` | `u32` | Device index (0, 1, 2...) | Required, unique |
| `model_name` | `String` | GPU model (e.g., "NVIDIA RTX 3080") | Required, max 100 chars |
| `vendor` | `Enum` | Vendor: NVIDIA, AMD, Intel, Other | Required |
| `opencl_capable` | `bool` | Whether GPU supports OpenCL | Required |
| `compute_capability` | `Option<String>` | Compute version (e.g., "8.6" for RTX 3080) | Optional |

**Relationships**:
- Has one `GPU Mining Stats` (1:1)
- Has one `GPU Health Metrics` (1:1)

**Backend Source of Truth**: GPU enumeration service (read-only, hardware-provided data)

**Lifecycle**:
- Created: On app startup via OpenCL device enumeration
- Updated: On hot-plug events (GPU added/removed)
- Deleted: When GPU becomes unavailable

**Example**:
```rust
pub struct GpuDevice {
    pub device_index: u32,
    pub model_name: String,
    pub vendor: GpuVendor,
    pub opencl_capable: bool,
    pub compute_capability: Option<String>,
}

pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}
```

---

### 2. GPU Mining Stats

**Purpose**: Represents mining performance metrics for a specific GPU

**Fields**:
| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| `gpu_device_index` | `u32` | Foreign key to GPU Device | Required |
| `current_hashrate` | `f64` | Current hashrate (H/s) | >= 0.0 |
| `lifetime_blocks_found` | `u64` | Total blocks mined by this GPU | >= 0 |
| `mining_uptime` | `Duration` | Time spent mining | >= 0 |
| `mining_status` | `Enum` | Status: Active, Idle, Error, Throttled | Required |
| `energy_efficiency` | `Option<f64>` | Hashrate/watt (H/W) | Optional, >= 0.0 |
| `thermal_efficiency` | `Option<f64>` | Hashrate/temperature (H/°C) | Optional, >= 0.0 |
| `throttle_percentage` | `u8` | Mining intensity (0-100%, 100% = full) | 0-100 |

**Relationships**:
- Belongs to one `GPU Device` (N:1)
- Requires `GPU Health Metrics` for efficiency calculations

**Backend Source of Truth**: `Arc<RwLock<MiningThreadPool>>` (Article XI)

**Persistence**: Lifetime blocks found saved to `~/.btpc/data/mining_stats_per_gpu.json`

**State Transitions**:
```
Idle → Active: User starts GPU mining
Active → Throttled: Temperature exceeds threshold
Throttled → Active: Temperature returns below (threshold - 5°C)
Active → Idle: User stops mining
Active → Error: GPU failure detected
Error → Idle: GPU reset/recovery
```

**Validation Rules**:
- If `power_consumption == 0` or unavailable → `energy_efficiency = None` (display "N/A")
- If `temperature == 0` or unavailable → `thermal_efficiency = None` (display "N/A")
- `throttle_percentage` reduced in 10% increments when overheating

**Example**:
```rust
pub struct GpuMiningStats {
    pub gpu_device_index: u32,
    pub current_hashrate: f64,
    pub lifetime_blocks_found: u64,
    pub mining_uptime: Duration,
    pub mining_status: GpuMiningStatus,
    pub energy_efficiency: Option<f64>,
    pub thermal_efficiency: Option<f64>,
    pub throttle_percentage: u8,
}

pub enum GpuMiningStatus {
    Active,
    Idle,
    Error(String),
    Throttled,
}
```

---

### 3. GPU Health Metrics

**Purpose**: Represents real-time hardware health data for a GPU

**Fields**:
| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| `gpu_device_index` | `u32` | Foreign key to GPU Device | Required |
| `temperature` | `Option<f32>` | Temperature (°C) | Optional, 0-150°C |
| `fan_speed` | `Option<u32>` | Fan speed (RPM) | Optional, >= 0 |
| `power_consumption` | `Option<f32>` | Power usage (Watts) | Optional, >= 0.0 |
| `memory_used` | `Option<u64>` | GPU memory used (MB) | Optional, >= 0 |
| `memory_total` | `Option<u64>` | GPU memory total (MB) | Optional, >= 0 |
| `core_clock_speed` | `Option<u32>` | GPU core clock (MHz) | Optional, >= 0 |
| `last_updated` | `Instant` | Timestamp of last sensor read | Required |

**Relationships**:
- Belongs to one `GPU Device` (N:1)

**Backend Source of Truth**: GPU monitoring service (hardware sensor polling via NVML/ADL)

**Update Frequency**: 5 seconds (conservative polling interval per user clarification)

**Graceful Degradation**: Individual metrics show `None` if sensor unavailable (display as "N/A")

**Validation Rules**:
- Temperature warning: `temp > user_configured_threshold` (default 80°C)
- Temperature critical: `temp > 95°C` (maximum configurable threshold)
- Memory used cannot exceed memory total (if both available)

**Example**:
```rust
pub struct GpuHealthMetrics {
    pub gpu_device_index: u32,
    pub temperature: Option<f32>,
    pub fan_speed: Option<u32>,
    pub power_consumption: Option<f32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub core_clock_speed: Option<u32>,
    pub last_updated: Instant,
}
```

---

### 4. Mining Page Sub-Tab State

**Purpose**: Represents UI navigation state for Mining page

**Fields**:
| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| `active_subtab` | `String` | Active sub-tab: "cpu-mining" or "gpu-mining" | Required, enum values only |
| `last_viewed_gpu` | `Option<u32>` | Last selected GPU device index | Optional, must exist in GPU list |

**Backend Source of Truth**: Frontend local state (UI-only, not authoritative)

**Persistence**: localStorage key: `btpc_active_tab_mining`

**Event-Driven**: Tab switches do not modify backend state (read-only dashboard)

**Validation Rules**:
- `active_subtab` must be one of: `"cpu-mining"`, `"gpu-mining"`
- `last_viewed_gpu` must reference valid GPU device index

**Example** (JavaScript):
```javascript
const miningPageState = {
  activeSubtab: "gpu-mining",  // "cpu-mining" | "gpu-mining"
  lastViewedGpu: 0              // GPU device index or null
};
```

---

## Aggregated Views

### GPU Dashboard View (Frontend)

**Purpose**: Combined view of all GPU data for UI rendering

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `devices` | `Vec<GpuDevice>` | All enumerated GPUs |
| `stats` | `HashMap<u32, GpuMiningStats>` | Per-GPU mining stats (keyed by device_index) |
| `health` | `HashMap<u32, GpuHealthMetrics>` | Per-GPU health metrics (keyed by device_index) |
| `temperature_threshold` | `f32` | User-configured warning threshold (°C) |

**Backend Event Payload**:
```rust
#[derive(Serialize)]
pub struct GpuDashboardData {
    pub devices: Vec<GpuDevice>,
    pub stats: HashMap<u32, GpuMiningStats>,
    pub health: HashMap<u32, GpuHealthMetrics>,
    pub temperature_threshold: f32,
}
```

**Tauri Event**: `"gpu-stats-updated"` emitted every 5 seconds

---

## Persistent Storage Schema

### ~/.btpc/data/mining_stats_per_gpu.json

**Format**:
```json
{
  "gpu_0": {
    "blocks_found": 42,
    "last_updated": "2025-11-11T10:00:00Z"
  },
  "gpu_1": {
    "blocks_found": 37,
    "last_updated": "2025-11-11T10:00:00Z"
  }
}
```

**Rust Struct**:
```rust
#[derive(Serialize, Deserialize)]
pub struct PersistedGpuStats {
    pub blocks_found: u64,
    pub last_updated: String,  // ISO 8601 timestamp
}

pub type MiningStatsStorage = HashMap<String, PersistedGpuStats>;
```

**Atomic Write Strategy**:
1. Write to temporary file: `mining_stats_per_gpu.json.tmp`
2. Fsync to ensure data on disk
3. Rename to `mining_stats_per_gpu.json` (atomic operation)

---

## Article XI Compliance

**Single Source of Truth** (Section 11.1):
- ✅ `GpuMiningStats`: Backend `MiningThreadPool` is authoritative
- ✅ `GpuHealthMetrics`: Backend sensor polling is authoritative
- ✅ `GpuDevice`: Backend enumeration service is authoritative
- ✅ `MiningPageSubTabState`: Frontend local state (UI-only, not authoritative)

**Event-Driven Architecture** (Section 11.3):
- ✅ Backend emits `"gpu-stats-updated"` event every 5 seconds
- ✅ Event payload contains full `GpuDashboardData` snapshot
- ✅ Frontend subscribes and updates UI reactively

**No Prohibited Patterns** (Section 11.7):
- ✅ No authoritative state in frontend JavaScript
- ✅ No frontend polling (events used instead)
- ✅ Temperature threshold: backend validates before localStorage save

---

**Data Model Complete**: 2025-11-11
**Next Phase**: Generate contracts/ directory with Tauri command schemas