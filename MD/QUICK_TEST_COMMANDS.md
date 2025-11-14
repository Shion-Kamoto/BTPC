# Quick Multi-Node Test Commands

## Start Nodes (Recommended Order)

### 1. Start Node1 (Mining Bootstrap)
```bash
cd /home/bob/BTPC/BTPC/testnet-deployment
nohup ../target/release/btpc_node --datadir data/node1 --rpcport 18350 \
  --listen 127.0.0.1:18351 --network testnet --mine > logs/node1.log 2>&1 &
echo "Node1 PID: $!"
```

### 2. Wait for Node1 to mine ~50 blocks (5 minutes)
```bash
# Monitor node1 height
watch -n 2 'curl -s http://127.0.0.1:18350 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"getblockchaininfo\",\"params\":[]}" | \
  python3 -m json.tool | grep -E "(blocks|bestblockhash)"'
```

### 3. Start Node2 (Sync Only - NO MINING)
```bash
nohup ../target/release/btpc_node --datadir data/node2 --rpcport 18360 \
  --listen 127.0.0.1:18361 --network testnet \
  --connect 127.0.0.1:18351 > logs/node2.log 2>&1 &
echo "Node2 PID: $!"
```

### 4. Start Node3 (Sync Only - NO MINING)
```bash
nohup ../target/release/btpc_node --datadir data/node3 --rpcport 18370 \
  --listen 127.0.0.1:18371 --network testnet \
  --connect 127.0.0.1:18351 > logs/node3.log 2>&1 &
echo "Node3 PID: $!"
```

## Verify Synchronization

### Check All Node Heights and Hashes
```bash
echo "=== Node Heights ===" && \
for port in 18350 18360 18370; do \
  echo -n "Port $port: " && \
  curl -s http://127.0.0.1:$port -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | \
  python3 -c "import json,sys; d=json.load(sys.stdin); \
    print(f\"Height {d['result']['blocks']}, Hash: {d['result']['bestblockhash'][:32]}...\")"
done
```

### Check P2P Handshakes
```bash
# Should see "Handshake complete" for all nodes
grep "Handshake" logs/node*.log | tail -10
```

### Check Block Exchange
```bash
# Should see "Received block" messages
grep "Received block" logs/node2.log | tail -5
grep "Received block" logs/node3.log | tail -5
```

## Success Criteria

✅ **Sync Working If**:
- All nodes show **identical** `bestblockhash`
- All nodes show **identical** `blocks` height
- Node2 and Node3 logs show "Received block" from node1
- NO "exceeds size limit" errors in logs

❌ **Sync Failing If**:
- Nodes have different `bestblockhash` values
- Heights differ by more than 1-2 blocks
- Logs show "exceeds size limit" errors
- Logs show "Handshake failed" messages

## Stop All Nodes
```bash
pkill -f btpc_node
ps aux | grep btpc_node | grep -v grep  # Verify stopped
```

## Clean Start (If Needed)
```bash
# Stop nodes
pkill -f btpc_node

# Clear blockchain data (keeps genesis)
cd /home/bob/BTPC/BTPC/testnet-deployment
rm -rf data/node*/blockchain

# Restart from step 1
```
