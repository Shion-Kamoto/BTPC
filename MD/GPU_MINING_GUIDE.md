# BTPC GPU Mining Guide
**Status**: Future Feature (Placeholder Implementation)
**Last Updated**: 2025-10-23

## Overview

BTPC supports GPU-accelerated mining through an optional `gpu` feature flag. This document describes how to enable GPU mining support and the current implementation status.

## Current Status

**⚠️ PLACEHOLDER IMPLEMENTATION**

The GPU mining feature is currently implemented as a placeholder with:
- ✅ Command-line interface (--gpu flag)
- ✅ OpenCL platform/device detection
- ✅ Configuration and initialization code
- ⚠️ CPU-fallback mining (no GPU acceleration yet)
- ❌ OpenCL SHA-512 kernel (future work)

**Why CPU-fallback?**
Implementing optimized SHA-512 mining kernels for OpenCL requires:
1. Low-level GPU programming in OpenCL C
2. Platform-specific optimizations (NVIDIA vs AMD)
3. Extensive testing across different GPU architectures
4. Performance tuning and benchmarking

## Building with GPU Support

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install ocl-icd-opencl-dev

# Fedora/RHEL
sudo dnf install ocl-icd-devel

# macOS (via Homebrew)
brew install opencl-headers
```

### Compile with GPU Feature

```bash
# Build btpc_miner with GPU support
cargo build --release --bin btpc_miner --features gpu

# Or build entire workspace with GPU
cargo build --release --features gpu
```

## Usage

### Check GPU Availability

```bash
# Run miner with GPU flag
./target/release/btpc_miner --gpu --network regtest

# Expected output (if OpenCL available):
✅ GPU mining enabled: OpenCL GPU device (simplified implementation)
⚠️  Note: Full GPU acceleration requires optimized OpenCL kernels
   Currently using CPU-fallback implementation
```

### Command-Line Options

```bash
btpc_miner [OPTIONS]

Options:
  -g, --gpu              Enable GPU mining (requires --features gpu at compile time)
  -t, --threads <COUNT>  Number of mining threads (default: auto-detect)
      --network <NET>    Network to mine on (mainnet, testnet, regtest)
      --rpc-url <URL>    RPC server URL (default: http://127.0.0.1:8332)
      --address <ADDR>   Mining address for rewards
      --message <MSG>    Coinbase message
  -h, --help            Print help
  -V, --version         Print version
```

## Configuration

### GPU Miner Config

```rust
use btpc_miner::gpu_miner::GpuMinerConfig;

let config = GpuMinerConfig {
    platform_id: 0,    // OpenCL platform index
    device_id: 0,      // OpenCL device index
    workgroup_size: 256, // GPU workgroup size
};
```

### Platform/Device Selection

```bash
# List available OpenCL platforms and devices (future feature)
btpc_miner --list-gpu-devices

# Select specific device (future feature)
btpc_miner --gpu --gpu-platform 0 --gpu-device 1
```

## Implementation Architecture

### Module Structure

```
bins/btpc_miner/
├── src/
│   ├── main.rs         # CLI and CPU mining loop
│   └── gpu_miner.rs    # GPU mining implementation
```

### GPU Miner API

```rust
// Create GPU miner
let config = GpuMinerConfig::default();
let gpu_miner = GpuMiner::new(config)?;

// Mine block (currently CPU-fallback)
let result = gpu_miner.mine_block(
    &header,
    &target,
    start_nonce,
    nonce_range,
)?;

// Get statistics
let total_hashes = gpu_miner.total_hashes();
let device_info = gpu_miner.device_info();
```

## Test-Driven Development

The GPU miner was developed following TDD (RED-GREEN-REFACTOR):

### Tests

```bash
# Run GPU miner tests
cargo test --package btpc_miner gpu_

# Expected: 5 tests, all passing (with CPU-fallback)
```

**Test Coverage**:
- ✅ GPU miner creation (graceful failure if no device)
- ✅ Block mining returns results
- ✅ Hash counter increments
- ✅ Workgroup size configuration
- ✅ Nonce finding logic

### RED Phase (Tests First)
All 5 tests were written before implementation, following Constitution v1.1 Article VI.3.

### GREEN Phase (Implementation)
CPU-fallback implementation passes all tests.

### REFACTOR Phase (Future Work)
Replace CPU-fallback with optimized OpenCL kernels.

## Performance Considerations

### Current (CPU-Fallback)
- Hash rate: ~2M H/s per core (SHA-512)
- Same as CPU mining
- No GPU acceleration

### Target (With OpenCL Kernels)
- Hash rate: 100M - 1000M H/s (depending on GPU)
- 50-500x speedup over CPU
- Power efficiency: 2-5x better than CPU

### Hardware Requirements
- **Minimum**: OpenCL 1.2 compatible GPU
- **Recommended**: Modern GPU with 4GB+ VRAM
- **Optimal**: NVIDIA RTX series or AMD RX 6000 series

## Implementation Roadmap

### Phase 1: Placeholder (✅ Complete)
- [x] Command-line interface
- [x] OpenCL initialization
- [x] Platform/device detection
- [x] CPU-fallback mining
- [x] Test suite

### Phase 2: SHA-512 Kernel (🔜 Future)
- [ ] Implement OpenCL SHA-512 kernel
- [ ] Optimize for NVIDIA GPUs
- [ ] Optimize for AMD GPUs
- [ ] Benchmark against CPU

### Phase 3: Advanced Features (🔮 Long-term)
- [ ] Multi-GPU support
- [ ] Automatic device selection
- [ ] Dynamic workgroup sizing
- [ ] Pool mining integration
- [ ] Monitoring and statistics

## Troubleshooting

### "GPU mining not available" Error

**Problem**: No OpenCL platforms found

**Solution**:
```bash
# Install OpenCL drivers
# NVIDIA
sudo apt-get install nvidia-opencl-icd

# AMD
sudo apt-get install mesa-opencl-icd

# Check installation
clinfo
```

### "GPU mining not enabled" Error

**Problem**: btpc_miner not compiled with GPU support

**Solution**:
```bash
cargo clean
cargo build --release --bin btpc_miner --features gpu
```

### OpenCL Platform Issues

**Problem**: Wrong platform/device selected

**Solution**:
```bash
# List devices (requires clinfo)
clinfo -l

# Modify GpuMinerConfig in code
let config = GpuMinerConfig {
    platform_id: 1,  // Try different platform
    device_id: 0,
    workgroup_size: 256,
};
```

## FAQ

### Q: Why use OpenCL instead of CUDA?
**A**: OpenCL is cross-platform (NVIDIA, AMD, Intel), while CUDA is NVIDIA-only. BTPC prioritizes accessibility and decentralization.

### Q: When will full GPU acceleration be available?
**A**: Timeline TBD. Depends on community demand and contributor availability. Estimated Q2 2026.

### Q: Can I help implement GPU kernels?
**A**: Yes! See CONTRIBUTING.md for guidelines. OpenCL/GPU programming experience required.

### Q: Is GPU mining profitable?
**A**: Profitability depends on:
- Network difficulty
- Electricity costs
- GPU hardware
- BTPC market price

Use mining calculators once network launches.

### Q: Does GPU mining work on macOS?
**A**: Theoretically yes (OpenCL supported), but:
- Apple deprecated OpenCL in favor of Metal
- Limited testing on macOS
- Metal implementation may be needed for optimal performance

## References

### OpenCL Resources
- [OpenCL Official](https://www.khronos.org/opencl/)
- [OpenCL Programming Guide](https://www.khronos.org/opencl/)
- [SHA-512 GPU Implementation](https://github.com/mjosaarinen/tiny_sha3) (reference)

### BTPC Documentation
- [Mining Guide](MINING_GUIDE.md)
- [Constitution](../CONSTITUTION.md)
- [Architecture](ARCHITECTURE.md)

## Contributing

To contribute GPU mining improvements:

1. **Fork and clone** the repository
2. **Enable GPU feature**: `cargo build --features gpu`
3. **Write tests** following TDD (RED-GREEN-REFACTOR)
4. **Implement OpenCL kernel** in `gpu_miner.rs`
5. **Benchmark** against CPU implementation
6. **Submit PR** with detailed performance metrics

See CONTRIBUTING.md for full guidelines.

---

**Status Summary**:
- ✅ Framework: Complete and tested
- ⚠️ Performance: CPU-fallback (no acceleration)
- 🔜 OpenCL Kernels: Future work
- 📝 Documentation: This guide

**Last Updated**: 2025-10-23 | **Version**: 0.1.0 (Placeholder)