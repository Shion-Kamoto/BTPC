# BTPC Testnet - Quick Reference Card

## Current Status (2025-10-05)

```
✅ Node Running:     PID 53442
✅ Test Running:     PID 108846 (3/288 checks)
✅ Height:           1103+ blocks
✅ Database:         7.2 MB
✅ RPC Status:       Fully functional
```

---

## Monitoring Commands

### Quick Health Check
```bash
./monitor-testnet.sh
```

### Watch Live Mining
```bash
tail -f /tmp/node1_final.log | grep "Block mined"
```

### Check Stress Test Progress
```bash
tail -f logs/stress-test-24hr.log
```

### Get Current Height
```bash
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
```

### Get Specific Block
```bash
# Replace <hash> with actual block hash
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblock","params":["<hash>"]}' | jq
```

---

## RPC Endpoints

**Base URL:** `http://127.0.0.1:18350`

### getblockchaininfo
```bash
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":"1","method":"getblockchaininfo","params":[]}' | jq
```

Returns:
```json
{
  "result": {
    "blocks": 1103,
    "bestblockhash": "4add9cb8e982da8a99ad9ce14d355b02...",
    "chain": "main",
    "difficulty": 1.0
  }
}
```

### getblock
```bash
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":"1","method":"getblock","params":["<hash>"]}' | jq
```

Returns:
```json
{
  "result": {
    "hash": "7458283a0e008f0173c259502c31b4729a82fea8...",
    "height": 175,
    "version": 1,
    "merkleroot": "...",
    "time": 1759621099,
    "nonce": 1,
    "bits": "207fffff",
    "difficulty": 1.0,
    "previousblockhash": "..."
  }
}
```

---

## Process Management

### Check If Running
```bash
ps aux | grep -E "btpc_node|stress-test" | grep -v grep
```

### Stop Node
```bash
pkill btpc_node
```

### Stop Stress Test
```bash
pkill -f stress-test-24hr.sh
```

### Restart Node
```bash
cd /home/bob/BTPC/BTPC/testnet-deployment
../target/release/btpc_node \
  --network testnet \
  --datadir data/node1 \
  --rpcport 18350 \
  --listen 127.0.0.1:18351 \
  --mine > /tmp/node1.log 2>&1 &
```

---

## Key Files

### Documentation
- `README.md` - Full deployment guide
- `STRESS_TEST_STATUS.md` - Test status
- `RPC_FIX_SUMMARY.md` - Bug fix details
- `SESSION_COMPLETION_SUMMARY.md` - Full session summary
- `QUICK_REFERENCE.md` - This file

### Logs
- `logs/stress-test-24hr.log` - Test metrics
- `/tmp/node1_final.log` - Node mining log
- `logs/monitor.log` - Monitoring history

### Data
- `data/node1/blockchain/` - Blockchain database
- `data/node1/genesis.json` - Genesis block
- `testnet-genesis.json/genesis.json` - Genesis source

### Scripts
- `monitor-testnet.sh` - Health monitoring
- `stress-test-24hr.sh` - 24-hour test
- `monitor-health.sh` - Detailed health check

---

## Important Fixes Applied

### RPC Height Bug (2025-10-05)
**Problem:** RPC always returned `height: 0`
**Fix:** Updated `btpc-core/src/rpc/handlers.rs` to query actual database
**Status:** ✅ FIXED

**Verification:**
```bash
# Before: always 0
# After: returns actual height
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
# Output: 1103 ✅
```

---

## Troubleshooting

### Node Not Mining
```bash
# Check if running
ps aux | grep btpc_node

# Check logs for errors
tail -100 /tmp/node1_final.log

# Look for "Mining started" in logs
grep "Mining started" /tmp/node1_final.log
```

### RPC Not Responding
```bash
# Test connection
curl http://127.0.0.1:18350

# Check if port is open
lsof -i :18350

# Verify node is running
ps aux | grep btpc_node
```

### Database Issues
```bash
# Check size
du -sh data/node1

# Check for corruption (no errors expected)
ls -la data/node1/blockchain/
```

---

## Expected Performance

- **Mining Rate:** ~1 block/second (testnet)
- **RPC Response:** < 100ms
- **CPU Usage:** 5-10%
- **Memory:** ~20-50 MB
- **Database Growth:** ~5-10 KB per block

---

## 24-Hour Stress Test

**Status:** Running (Check 3/288)
**Start:** 2025-10-05 10:15:18
**End:** 2025-10-06 10:15:18

**What It Tests:**
- Continuous block production
- RPC stability
- Database growth
- System resources
- Error detection

**Monitor Progress:**
```bash
# Watch test log
tail -f logs/stress-test-24hr.log

# Check current metrics
./monitor-testnet.sh

# View test status
cat STRESS_TEST_STATUS.md
```

---

## Quick Verification Checklist

- [ ] Node running? `ps aux | grep btpc_node`
- [ ] Mining blocks? `tail -f /tmp/node1_final.log | grep "Block mined"`
- [ ] RPC working? `curl http://127.0.0.1:18350`
- [ ] Height increasing? `./monitor-testnet.sh`
- [ ] No errors? `grep -i error /tmp/node1_final.log`
- [ ] Test running? `ps aux | grep stress-test`

---

**All systems operational! 🚀**

For detailed information, see the full documentation files listed above.
