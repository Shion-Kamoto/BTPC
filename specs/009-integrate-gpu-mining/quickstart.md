# Quickstart: GPU Mining Stats Testing

**Feature**: 009-integrate-gpu-mining Phase 3
**Date**: 2025-11-09

## Prerequisites

- ✅ Phase 2 complete (GPU mining functional)
- ✅ OpenCL-compatible GPU installed
- ✅ `btpc_miner` built with `--features gpu`
- ✅ Desktop app development environment ready

## Manual Test Scenario

### Terminal 1: Start GPU Miner with Stats Server

```bash
# Start miner with GPU enabled and stats server
/home/bob/.btpc/bin/btpc_miner --gpu --network regtest --address test_address
```

**Expected Output**:
```
✅ GPU mining enabled: AMD Radeon RX 6800 (64 compute units, 16.0 GB VRAM)
📊 GPU stats server listening on http://127.0.0.1:18360
⛏️  Mining started on 8 CPU cores + GPU
```

**Verification**:
- No compilation errors
- Stats server starts without panics
- Port 8333 listening (check with `netstat -tuln | grep 8333`)

---

### Terminal 2: Query Stats Endpoint

```bash
# Test /stats endpoint
curl http://127.0.0.1:18360/stats | jq
```

**Expected JSON Response**:
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

**Verification**:
- ✅ HTTP 200 OK response
- ✅ Valid JSON structure matching GpuStats schema
- ✅ `hashrate` > 0 (indicates mining is active)
- ✅ `total_hashes` increasing over time (query multiple times)
- ✅ `uptime_seconds` increasing linearly
- ✅ `compute_units` matches known GPU specs
- ✅ `global_mem_size` in bytes (e.g., 16GB = 17179869184 bytes)

```bash
# Test /health endpoint
curl http://127.0.0.1:18360/health | jq
```

**Expected Response**:
```json
{
  "status": "ok",
  "service": "btpc_miner_stats"
}
```

**Verification**:
- ✅ HTTP 200 OK response
- ✅ `status` field equals "ok"

---

### Terminal 3: Desktop App Integration Test

```bash
# Launch desktop app in development mode
cd btpc-desktop-app && npm run tauri:dev
```

**In Desktop App UI**:

1. **Navigate to Mining Tab**
   - Click "Mining" in navigation menu
   - Verify tab loads without errors

2. **Start GPU Mining**
   - Click "Start Mining" button
   - Enable "Use GPU" checkbox
   - Observe mining status changes to "Running"

3. **Verify GPU Stats Panel**
   - Panel should appear below mining controls
   - Check displayed values match expected data:

   ```
   GPU Stats
   ─────────────────────────────────
   Device:       AMD Radeon RX 6800
   Vendor:       Advanced Micro Devices
   Hashrate:     342.5 MH/s
   Compute Units: 64
   VRAM:         16.0 GB
   Clock Speed:  2105 MHz
   Uptime:       00:01:00
   Total Hashes: 20,550,000,000
   ```

4. **Real-Time Updates**
   - Wait 5-10 seconds
   - Verify hashrate updates (should fluctuate slightly)
   - Verify uptime increments
   - Verify total_hashes increases

5. **Error Handling Test**
   - Stop miner (kill Terminal 1 process)
   - Observe desktop app shows "GPU stats unavailable" message
   - No crashes or unhandled exceptions

---

## Expected Performance

### Stats Query Latency

```bash
# Benchmark stats endpoint response time
time curl -s http://127.0.0.1:18360/stats > /dev/null
```

**Expected Result**: < 10ms (local HTTP request + OpenCL query)

### Mining Overhead

Monitor hashrate before/after enabling stats server:

```bash
# Baseline: GPU mining without stats server
# Run 1: Note hashrate from miner logs

# With stats: GPU mining + stats server
# Run 2: Note hashrate from /stats endpoint

# Overhead = (Run1 - Run2) / Run1 * 100%
```

**Expected Result**: < 1% hashrate degradation (stats queries should be negligible)

---

## Troubleshooting

### Stats Server Not Starting

**Symptom**: No "📊 GPU stats server listening" message

**Debug Steps**:
1. Check port 8333 availability: `netstat -tuln | grep 8333`
2. Review miner logs for warp errors
3. Verify `warp` dependency in Cargo.toml
4. Check firewall rules (should allow localhost:18360)

### GPU Stats Return Error

**Symptom**: `curl` returns `{"error": "GPU mining not enabled"}`

**Debug Steps**:
1. Verify `--gpu` flag passed to miner
2. Check OpenCL installation: `clinfo` (should list GPUs)
3. Review miner startup logs for GPU detection errors
4. Verify `ocl` feature enabled: `cargo build --features gpu`

### Desktop App Shows "Stats Unavailable"

**Symptom**: UI displays error message instead of stats

**Debug Steps**:
1. Verify miner is running (Terminal 1 process active)
2. Test stats endpoint manually: `curl http://127.0.0.1:18360/stats`
3. Check browser console for fetch errors (CORS, network)
4. Verify Tauri command registered in `main.rs`
5. Check `reqwest` dependency in desktop app Cargo.toml

### Hashrate Always Zero

**Symptom**: `"hashrate": 0.0` despite mining running

**Debug Steps**:
1. Check `uptime_seconds`: if 0, mining just started (wait 5-10 seconds)
2. Verify `total_hashes` is increasing
3. Review `GpuMiner.get_stats()` calculation logic
4. Check for integer overflow in hashrate calculation

---

## Success Criteria

✅ **Functional**:
- Stats server starts without errors
- `/stats` endpoint returns valid JSON
- Desktop app displays GPU stats in real-time
- Stats update every 1-2 seconds

✅ **Performance**:
- Stats query latency < 10ms
- Mining overhead < 1%
- No UI lag when polling stats

✅ **Reliability**:
- No panics or crashes over 10 minute test
- Graceful error handling when miner stops
- Accurate hashrate calculations (matches miner logs ±5%)

---

## Next Steps

After successful quickstart validation:

1. **Run Integration Tests**: `cargo test --test test_gpu_stats_endpoint`
2. **Performance Profiling**: Use `perf` to validate stats overhead
3. **Desktop App Polish**: Add charts, historical data, alerts
4. **Multi-GPU Support**: Extend for multiple GPUs (future phase)

---

**References**:
- Plan: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/plan.md`
- API Contract: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/contracts/gpu-stats-api.yaml`
- Data Model: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/data-model.md`