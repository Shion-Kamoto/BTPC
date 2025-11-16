# Data Model: GPU Mining Stats

**Feature**: 009-integrate-gpu-mining Phase 3
**Date**: 2025-11-09

## Entity: GpuStats

### Structure

```rust
pub struct GpuStats {
    pub device_name: String,       // e.g., "AMD Radeon RX 6800"
    pub vendor: String,             // e.g., "Advanced Micro Devices"
    pub compute_units: u32,         // CUs/SMs
    pub max_work_group_size: usize, // Max threads per workgroup
    pub global_mem_size: u64,       // VRAM in bytes
    pub local_mem_size: u64,        // Local mem in bytes
    pub max_clock_frequency: u32,   // MHz
    pub hashrate: f64,              // MH/s (calculated)
    pub total_hashes: u64,          // Lifetime hashes
    pub uptime_seconds: u64,        // Mining uptime
    pub temperature: Option<f32>,   // °C (if available)
    pub power_usage: Option<f32>,   // W (if available)
}
```

### Field Descriptions

- **device_name**: OpenCL device name string (CL_DEVICE_NAME)
- **vendor**: GPU manufacturer (CL_DEVICE_VENDOR)
- **compute_units**: Number of compute units (NVIDIA: SMs, AMD: CUs) (CL_DEVICE_MAX_COMPUTE_UNITS)
- **max_work_group_size**: Maximum workgroup size for kernel execution (CL_DEVICE_MAX_WORK_GROUP_SIZE)
- **global_mem_size**: Total VRAM in bytes (CL_DEVICE_GLOBAL_MEM_SIZE)
- **local_mem_size**: Local/shared memory per workgroup in bytes (CL_DEVICE_LOCAL_MEM_SIZE)
- **max_clock_frequency**: GPU core clock frequency in MHz (CL_DEVICE_MAX_CLOCK_FREQUENCY)
- **hashrate**: Real-time hashrate in MH/s (million hashes per second), calculated value
- **total_hashes**: Cumulative hash count since mining started
- **uptime_seconds**: Time elapsed since mining started in seconds
- **temperature**: GPU temperature in Celsius (Optional - requires vendor extensions)
- **power_usage**: GPU power consumption in Watts (Optional - requires vendor extensions)

### Validation Rules

1. **device_name**: Must be non-empty OpenCL device string
2. **vendor**: Must be non-empty vendor string
3. **compute_units**: Must be > 0 (valid GPU has at least 1 compute unit)
4. **hashrate**: Calculated as `total_hashes / uptime_seconds / 1_000_000.0`
   - Returns 0.0 if uptime_seconds == 0 (prevents division by zero)
5. **total_hashes**: Monotonically increasing counter (never decreases)
6. **uptime_seconds**: Monotonically increasing counter (never decreases)
7. **temperature**: Optional field (None if unavailable via standard OpenCL)
8. **power_usage**: Optional field (None if unavailable via standard OpenCL)

### Data Sources

- **OpenCL DeviceInfo API**: All required fields (device_name through max_clock_frequency)
- **Mining Runtime**: hashrate, total_hashes, uptime_seconds (calculated from GpuMiner state)
- **Vendor Extensions**: temperature, power_usage (future enhancement via NVIDIA NVML or AMD ADL)

### Serialization

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStats { /* ... */ }
```

- **Format**: JSON (via serde_json)
- **Transport**: HTTP response body for `/stats` endpoint
- **Encoding**: UTF-8

### Example Instance

```json
{
  "device_name": "AMD Radeon RX 6800",
  "vendor": "Advanced Micro Devices",
  "compute_units": 64,
  "max_work_group_size": 256,
  "global_mem_size": 17179869184,
  "local_mem_size": 65536,
  "max_clock_frequency": 2105,
  "hashrate": 342.5,
  "total_hashes": 20550000000,
  "uptime_seconds": 60,
  "temperature": null,
  "power_usage": null
}
```

### State Transitions

GpuStats is a **read-only snapshot** - no state machine required.

Stats are queried on-demand via `GpuMiner.get_stats()` which:
1. Queries OpenCL DeviceInfo (static device properties)
2. Reads mining runtime counters (total_hashes, start_time)
3. Calculates derived values (hashrate, uptime_seconds)
4. Returns immutable GpuStats instance

### Thread Safety

- **Reads**: Thread-safe (OpenCL DeviceInfo queries use internal driver locking)
- **No Writes**: GpuStats is constructed on-demand, never mutated
- **Sharing**: Safe to clone and send across threads (Clone + Send + Sync)

### Performance Characteristics

- **Query Time**: < 0.1ms (OpenCL DeviceInfo calls are driver-cached)
- **Memory Size**: ~152 bytes (stack-allocated struct)
- **Serialization**: < 1ms (simple JSON encoding)

---

**References**:
- OpenCL 1.2 Specification § 4.2 (Querying Devices)
- Implementation: `bins/btpc_miner/src/gpu_miner.rs:29-42`