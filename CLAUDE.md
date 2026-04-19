# BTPC Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-15

## Active Technologies
- Rust 1.75+ (workspace `btpc-core`, `src-tauri`); no UI changes in this follow-up + Tokio 1.x (async), existing `SimplePeerManager`, existing `IntegratedSyncManager`, `BlockSource` trait (already defined in `btpc-core/src/network/block_source.rs` per T053) (003-testnet-p2p-hardening)
- Existing RocksDB unified database (`src-tauri/src/unified_database.rs`) — no new column families, no schema change (003-testnet-p2p-hardening)

- Rust 1.75+ (workspace `btpc-core`, `src-tauri`); vanilla JavaScript (ES2022) + HTML5 + CSS for the UI layer (no framework). + Tokio (async runtime), Tauri 2.x (desktop shell/IPC), `btpc-core` workspace crate, `serde` / `serde_json` for IPC payloads, existing `ConnectionTracker`, `PeerBanManager`, `RateLimiter`, `SimplePeerManager`, `IntegratedSyncManager`. (003-testnet-p2p-hardening)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 1.75+ (workspace `btpc-core`, `src-tauri`); vanilla JavaScript (ES2022) + HTML5 + CSS for the UI layer (no framework).: Follow standard conventions

## Recent Changes
- 003-testnet-p2p-hardening: Added Rust 1.75+ (workspace `btpc-core`, `src-tauri`); no UI changes in this follow-up + Tokio 1.x (async), existing `SimplePeerManager`, existing `IntegratedSyncManager`, `BlockSource` trait (already defined in `btpc-core/src/network/block_source.rs` per T053)

- 003-testnet-p2p-hardening: `BlockchainEvent::SyncProgressUpdated` is now the single authoritative node-status channel. Per-page polling is deprecated. All UI pages subscribe via `btpc-node-status.js`. `PeerListUpdated` pushes the peer table on connect/disconnect. Tauri commands: `get_shared_node_status`, `list_peers`, `ban_peer`, `disconnect_peer`, `add_node`, `get_network_info`. `IntegratedSyncManager` implements headers-first sync with `InFlightTracker` (per-peer cap 16), `BlockRequestScheduler` (aggregate cap 128), and `StallReaper` (30s timeout, +10 ban).

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
