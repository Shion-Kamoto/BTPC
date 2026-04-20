# Status 3 — Testnet P2P Hardening (spec 003)

**Date:** 2026-04-15
**Branch:** `003-testnet-p2p-hardening`
**Remote:** `origin/003-testnet-p2p-hardening` @ `89cf4ed`

---

## Where we are

Spec-Kit feature `003-testnet-p2p-hardening` is in **Phase 3 GREEN US1** with two
UI polish commits layered on top. The underlying P2P block-sync path
(**Problem A / Story 3 / US3**) has **not** been re-ported yet — this is the
primary blocker preventing Machine A's local chain from actually advancing past
the genesis block.

## Commits on the feature branch (ahead of `main`)

| SHA | Title | What it does |
|---|---|---|
| `89cf4ed` | feat(ui): network connection indicator on Peers tab | Red/amber/green status banner at top of Peers tab, driven by the existing 5s updateManager cycle |
| `c819463` | fix(ui): display network block height instead of local tip (Fix B) | Dashboard + Node > Blockchain Info tiles now show `network_height` (max peer tip) instead of local tip; `get_sync_progress` fixed to compute real `target_height` from peers |
| `b708183` | feat(network): testnet P2P hardening US1 — address manager, ban persistence, rate limits | Phase 3 GREEN US1 production scaffolding: `AddressManager` (addrman-style bucketing), `BanStorage` (RocksDB), per-MessageType rate limits, `handle_getaddr`/`handle_inbound_addr`, Peer metadata struct, `HandshakeOutcome` self-connect detection stub, US3 compile-level stubs in `integrated_sync.rs` |

## What is working

- **Machine B (local, fully synced)** — renders correct block height, peer list,
  sync progress, and connection indicator. Has been the stable reference throughout.
- **`cargo check -p btpc-desktop-app --lib`** — GREEN. Compiles cleanly.
- **`cargo check -p btpc-core`** — GREEN.
- **Fix B logic** — on Machine A it is **expected** to show network height
  (e.g. `6,310`) as the primary Dashboard figure with "Local: 0 (syncing)"
  subtext in warning color.
- **Network indicator** — expected to flip red → amber → green based on
  `peer_count` and `local < network` gap.

## What is NOT working (and why)

### 1. Block sync — local chain stuck at 0 on any catching-up node

**Root cause:** Headers-first block sync was reverted from the live flat tree
at commit `eb9a53e`. The naive sync path in `btpc-core/src/network/sync.rs:75-88`
currently says:

> *"For a local testnet…immediately transition to synced state"*

So the node marks itself "synced" at height 0 as soon as it boots and **never
asks peers for blocks**. Peer connections, handshakes, and version messages all
work — but there is no block request pipeline running.

**The UI symptom this causes:**
- Local tip stays at 0 forever
- Best block hash stays at genesis
- Mining difficulty stays at genesis (`0x3c7fffff` → `1,015,021,567`) instead
  of the real network difficulty at tip (`0x3c2b70c2` → `1,009,479,874`)

**Fix path:** Re-port headers-first sync from the inert `btpc-desktop-app/`
tree (commit `af3f9a4`) into the flat tree. This is **Story 3 / US3 /
research R11** in the spec-kit plan. It requires:
- `btpc-core/src/network/integrated_sync.rs` — replace `todo!()` stubs with
  real bounded in-flight window + stall reaper
- `btpc-core/src/network/sync.rs` — remove the "immediately synced" shortcut
- `btpc-core/src/network/block_source.rs` (re-port from inert tree)
- Wire block/header request/response into the live peer message loop

### 2. Mining difficulty display on catching-up nodes

**Deferred** per user decision. Will self-correct once Problem A is fixed
(once `blockchain_state.difficulty_bits` reflects a real synced tip, the
existing display code is already correct). No UI or backend work needed for
this until sync is restored.

### 3. Pre-existing US4 RED-test module broken

`src-tauri/src/embedded_node.rs:3168+` contains a test module referencing
`build_node_status_snapshot` and `EmbeddedNode::test_instance` that do not
exist. This is **pre-existing** from earlier RED-phase US4 work and does
**not** block `cargo check --lib`. It only surfaces under
`cargo check --tests`. Out of scope for the current session.

## Outstanding diagnostic questions (awaiting user input)

Machine A was reported as "not syncing and no updated node data being
displayed." Before I can tell whether this is the expected
"Problem A still unfixed" state vs. a real regression, I need six answers:

1. **Which machine?** A or B.
2. **Verified pull?** Does `git log --oneline -3` show `89cf4ed` at the top?
3. **Node started?** RUNNING or OFFLINE on the Node page.
4. **Dashboard BLOCK HEIGHT tile** — exact number and subtext.
5. **Peers tab connection banner** — red/amber/green, text next to the dot,
   peer list count.
6. **Machine B status** — is it running and reachable from Machine A's network?

Answers will classify the issue as:
- **(a)** Pull/rebuild didn't land → retry git fetch + rebuild
- **(b)** UI landed but no peers connecting → networking/firewall/address issue
- **(c)** UI landed, peers connecting, height stuck at 0 locally → **expected**,
  proceed with Story 3 sync re-port
- **(d)** Something unexpected

## Deferred work (intentionally parked)

- **T018–T024** — wire `AddressManager` into `SimplePeerManager` outbound
  selection path, wire `handle_getaddr`/`handle_inbound_addr` into live
  `handle_peer_message` switch
- **T028** — bridge `query_dns_seeds` → `AddressManager` (auto-discovery)
- **T029** — `btpc-core/examples/headless_node.rs` for manual interop
- **T030** — promote `testing.rs` TestHarness from `todo!()` stubs to real
  two-node localhost runtime
- **T031–T032b** — fuzz harness, stall recovery runtime, throughput harness,
  cold-start trial runner
- **`[::]` address leak** — peer advertising its own bind-all address
- **Self-connect nonce check** — wire `handle_inbound_version` into live
  handshake path so `[::1]:...` self-connections disconnect
- **Peers tab refresh button also refreshing blockchain info**

## Files NOT committed to git (intentionally)

Per user rules — these live in the working tree but are not pushed:

- `btpc-core/tests/` — RED-phase tests
- `ui/tests/` — RED-phase tests
- `BTPC-new-backup-34/BTPC/` — inert backup tree
- `.github/` — unclear origin
- `.specify/` — spec-kit templates (mix of valid and nested-duplicate dirs)
- `CLAUDE.md` — auto-generated by `update-agent-context.sh`
- `spec-kit/` — full untracked dir, unclear origin
- `specs/` — feature spec directories

## Immediate next step

**Wait for user diagnostic answers** (see list above). Once we know whether
Machine A is in state (a)/(b)/(c)/(d), proceed accordingly.

If state (c) (expected stuck-at-0), the next major work item is **Story 3 /
US3 headers-first sync re-port** — the single change that will make Machine A
and every other testnet node actually download blocks, sync to the network
tip, and display consistent chain data across every instance of the app.

---

## Update — 2026-04-20 — Machine A post-fix test run

After PR #8 (`0fde91b` + `1fcfd70`) merged and pulled on Machine A:

- **Result:** node connection failed. Machine A did not reach a running state
  against Machine B — catch-up could not be observed.
- Log capture deferred; user will provide Machine A's log tomorrow.
- Commits on the branch that were under test:
  - `0fde91b fix(p2p): wire Headers/GetHeaders through live PeerEvent loop`
    (PeerEvent variants, MessageSender callback, UnifiedDatabaseBlockSource,
    EmbeddedNode arms for HeadersReceived / GetHeadersRequested, per-peer
    GetData cap 16).
  - `1fcfd70 fix(btpc_node): add HeadersReceived/GetHeadersRequested arms`
    (resolved non-exhaustive match surfaced by Machine A's build of the
    standalone `bins/btpc_node` binary).
- **Pending diagnosis:** log arrives tomorrow. Candidate failure modes to
  check first: outbound dial to Machine B refused, handshake aborted before
  GetHeaders, `UnifiedDatabaseBlockSource` lookup panics, or the new arms
  never firing (dispatch loop still using an older build somewhere in the
  Machine A layout).
- T120 (manual validation) **NOT** satisfied; per Art. V §II the fix
  stays OPEN until Machine A's Dashboard height is visually confirmed
  climbing toward Machine B's tip.

---

## Update — 2026-04-20 — Bugs #1–#5 landed on branch

After Machine A's post-PR-#8 log was captured, five distinct P2P
bugs were identified and fixed in sequence. All five are compiled,
committed, and pushed to `origin/003-testnet-p2p-hardening`.

**Branch tip:** `2b1b747` (ahead of `1a7352e`, `eee54ac`, `1fcfd70`, `0fde91b`).

### Bug #1 — single-hash locator (`eee54ac`)

`PeerConnected` sent a locator containing only our current tip hash.
On a diverged chain `find_fork_point` returned 0, forcing a full
2000-header replay from genesis that crashed Machine A.

**Fix:** module-level `build_block_locator(database, tip_height)`
helper in `src-tauri/src/embedded_node.rs` that emits a proper
Bitcoin-style exponentially-spaced locator (step doubles after 10
entries). Called from the `PeerConnected` handler.

### Bug #2 — no Headers chaining (`eee54ac`)

`HeadersReceived` requested the first 16 blocks but never asked for
the *next* batch of headers — sync stalled after one round-trip.

**Fix:** when `headers.len() >= HEADERS_BATCH_MAX` (2000), chain a
fresh `GetHeaders` using `[last_header.hash(), ...build_block_locator]`
as the locator, immediately after the GetData send.

### Bug #3 — addr-relay storm (`eee54ac`)

Addr relay `≤10` path had no dedup — each relay bounced between peers
exponentially until the 100-msg/sec rate limiter disconnected both
sides within seconds.

**Fix:** process-global `OnceLock<Mutex<HashMap<SocketAddr, Instant>>>`
dedup table in `btpc-core/src/network/simple_peer_manager.rs`
(`dedup_addrs_for_relay`, 60 s TTL, 1000 cap). Filters already-relayed
addresses before rebroadcast.

### Bug #4 — headers size cap too small (`1a7352e`)

`Message type 'headers' exceeds size limit: 723392 bytes (max: 162000
bytes)` — Bitcoin-sized cap (2000 × 81 B) does not fit post-quantum
headers carrying Dilithium5 signatures (~361 B each, observed ~4.5× the
Bitcoin envelope).

**Fix:** `MAX_HEADERS_MESSAGE_SIZE` in
`btpc-core/src/network/protocol.rs` raised to `2_000 * 512` (~1 MB).
Test assertion updated accordingly.

### Bug #5 — GetData handler missing (`2b1b747`)

After Bugs #1–#4 Machine A received **all** 6519 headers in 4 batches
but its height stayed at 0 because the serving side silently dropped
every GetData request. `SimplePeerManager::handle_peer_message` had
no match arm for `Message::GetData`.

**Three coordinated changes:**

- `btpc-core/src/network/simple_peer_manager.rs` — new
  `PeerEvent::GetDataRequested { from, inv }` variant; new
  `Message::GetData(inv)` arm in `handle_peer_message` that logs
  `📥 GetData from <peer> — N item(s) requested` and emits the event.
- `src-tauri/src/embedded_node.rs` — event handler iterates inv,
  filters `InvType::Block`, looks each hash up via
  `UnifiedDatabaseBlockSource::get_block_by_hash`, replies with
  `Message::Block` for hits and a single batched `Message::NotFound`
  for misses. Logs `📦 Served N block(s) to peer <peer> (M not found)`.
- `bins/btpc_node/src/main.rs` — matching non-exhaustive arm
  (headless node has no BlockSource, just logs the request).

### Compile status

- `cargo check -p btpc-core` — GREEN
- `cargo check -p btpc-desktop-app --lib` — GREEN
- `cargo check -p btpc_node` — GREEN

### Immediate next step

Two-machine test. Expected log pattern once working:

- **Machine B (serving):** `📥 GetData from <A> — 16 item(s) requested`
  → `📦 Served 16 block(s) to peer <A> (0 not found)`
- **Machine A (catching up):** `BlockReceived` fires, height climbs
  from 0 → 16 → 32 → … in scheduler-window steps tracking Machine B's
  tip (~6519).

T120 remains OPEN until height advance is visually confirmed.

### Deferred (acknowledged, not in scope)

- Machine B Peers tab: "No peers connected" while peer counter shows 6
  (UI/data inconsistency on the Peers page)
- Block-download scheduler requests disjoint 16-block windows rather
  than sequential — works, but looks odd in logs. Will revisit after
  T120 closes.
