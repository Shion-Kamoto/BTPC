# Feature Specification: BTPC Desktop UI Integration

**Feature ID**: 002
**Status**: In Progress
**Created**: 2025-10-02
**Network Support**: Mainnet, Testnet, Regtest

## Summary

Enhance the existing BTPC Desktop UI (Tauri application) to provide full operational control over the BTPC node, miner, and wallet with support for mainnet, testnet, and regtest networks.

## Problem Statement

The current BTPC Desktop UI exists but:
1. References old binary names that don't match the implemented binaries
2. Lacks integration with btpc_node for blockchain synchronization
3. Cannot manage background processes (node, miner)
4. Network switching is UI-only, doesn't reconfigure backend services
5. No real-time monitoring of node/miner status

Users need a unified interface to run a full BTPC stack on any network.

## Functional Requirements

### FR1: Node Management
- **FR1.1**: Start/stop btpc_node as a background process
- **FR1.2**: Display real-time node status (syncing, synced, stopped)
- **FR1.3**: Show peer count, current block height, sync progress
- **FR1.4**: Configure node settings (RPC port, P2P port, data directory)
- **FR1.5**: View node logs

### FR2: Miner Management
- **FR2.1**: Start/stop btpc_miner as a background process
- **FR2.2**: Display real-time mining statistics (hashrate, blocks found)
- **FR2.3**: Configure mining threads, mining address
- **FR2.4**: Show cumulative mining stats

### FR3: Wallet Management
- **FR3.1**: Create/import wallets
- **FR3.2**: View balance and transaction history
- **FR3.3**: Send BTPC transactions
- **FR3.4**: Generate receiving addresses
- **FR3.5**: Export wallet backup

### FR4: Network Switching
- **FR4.1**: Switch between Mainnet, Testnet, Regtest
- **FR4.2**: Auto-stop all services when switching networks
- **FR4.3**: Reconfigure services for new network
- **FR4.4**: Maintain separate data directories per network

### FR5: Dashboard Integration
- **FR5.1**: Show aggregate status (node, miner, wallet)
- **FR5.2**: Display key metrics (balance, hashrate, block height)
- **FR5.3**: Quick action buttons (start node, start mining)

## Technical Requirements

### TR1: Backend Architecture
- **Language**: Rust (Tauri backend)
- **Process Management**: tokio async processes
- **Binary Integration**: btpc_node, btpc_wallet, btpc_miner from /home/bob/BTPC/BTPC/target/release/
- **State Management**: Shared Arc<Mutex<AppState>>

### TR2: Frontend Architecture
- **Framework**: Vanilla JavaScript (existing)
- **Communication**: Tauri IPC commands
- **Real-time Updates**: Polling or Tauri events
- **Styling**: Existing CSS framework

### TR3: Data Storage
- **Configuration**: ~/.btpc/ui-config.toml
- **Network Data**: Separate dirs: ~/.btpc/mainnet/, ~/.btpc/testnet/, ~/.btpc/regtest/
- **Logs**: ~/.btpc/logs/

## User Stories

### US1: Run Full Node
**As a** BTPC user
**I want to** start a full node from the UI
**So that** I can participate in the network and validate transactions

**Acceptance Criteria**:
- Click "Start Node" button
- Node process starts in background
- UI shows "Syncing..." status
- Peer count and block height update in real-time
- Can stop node cleanly

### US2: Mine BTPC
**As a** BTPC miner
**I want to** start mining from the UI
**So that** I can earn block rewards

**Acceptance Criteria**:
- Click "Start Mining" button
- Miner connects to local node
- UI shows current hashrate
- Mining statistics update every 10 seconds
- Can stop miner anytime

### US3: Switch Networks
**As a** BTPC developer
**I want to** switch between mainnet and testnet
**So that** I can test without risking real funds

**Acceptance Criteria**:
- Select "Testnet" from network dropdown
- All running services stop gracefully
- UI reconfigures for testnet
- Data directory switches to ~/.btpc/testnet/
- Can start services on new network

### US4: Send Transaction
**As a** BTPC wallet user
**I want to** send BTPC to another address
**So that** I can transfer value

**Acceptance Criteria**:
- Enter recipient address and amount
- Transaction is created and signed
- Shows confirmation dialog
- Transaction broadcasts to network (via node)
- Balance updates after confirmation

## Non-Functional Requirements

### NFR1: Performance
- UI remains responsive during blockchain sync
- Mining statistics update with <1s latency
- Node start time <5 seconds

### NFR2: Reliability
- Graceful shutdown of all processes on app close
- Process crash recovery (auto-restart option)
- No data loss on unexpected shutdown

### NFR3: Security
- Wallet passwords encrypted with argon2
- Private keys never exposed to frontend
- RPC communication over localhost only

### NFR4: Usability
- One-click start for all services
- Clear status indicators (running/stopped/error)
- Helpful error messages with recovery steps

## Success Criteria

1. ✅ User can run full node on mainnet, testnet, and regtest
2. ✅ User can mine blocks and see rewards in wallet
3. ✅ User can send/receive transactions
4. ✅ Network switching works without manual configuration
5. ✅ All processes shut down cleanly
6. ✅ UI shows accurate real-time status

## Out of Scope

- Mobile app version
- Multi-wallet support (v2 feature)
- Advanced mining pool configuration
- Blockchain explorer with transaction search
- Multi-language support

## Dependencies

- btpc-core library (completed)
- btpc_node binary (completed)
- btpc_wallet binary (completed)
- btpc_miner binary (completed)
- Tauri 2.0 framework (installed)

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Process management complexity | High | Use proven tokio patterns, add comprehensive error handling |
| Network switching edge cases | Medium | Thorough testing on all networks, state validation |
| UI/backend synchronization | Medium | Use Tauri events for push updates, fallback polling |
| Resource usage (node + miner) | Medium | Add resource monitoring, configurable limits |

## Timeline Estimate

- Backend process management: 4 hours
- Node Management UI: 2 hours
- Mining integration: 2 hours
- Network switching: 2 hours
- Testing & polish: 2 hours
**Total**: ~12 hours

## Implementation Phases

**Phase 1**: Backend Infrastructure (process_manager.rs, updated btpc_integration.rs)
**Phase 2**: Tauri Commands (node/miner/wallet control)
**Phase 3**: Frontend UI (Node tab, updated Mining tab)
**Phase 4**: Integration & Testing

---

**Prepared by**: Claude Code
**Approved by**: Pending user review
**Implementation Start**: 2025-10-02