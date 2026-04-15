//! T099 / T108 RED-phase — Tauri command contract tests for shared node visibility.
//!
//! Production body is intentionally empty. The `#[cfg(test)]` module below
//! references symbols that do NOT yet exist — this is RED evidence per
//! Constitution Art. V §III (TDD RED-first NON-NEGOTIABLE).
//!
//! GREEN implementation lands in Phase 4 (US2) and Phase 6 (US4) and will
//! introduce:
//!   - `get_node_status`   command returning `NodeStatusDto`
//!   - `list_peers`        command returning `Vec<PeerSummary>`
//!   - `ban_peer`          command (takes address, duration)
//!   - `disconnect_peer`   command (takes address)
//!   - `add_node`          command (takes host:port)
//!   - `get_network_info`  command returning chain/network summary

#[cfg(test)]
mod contract_tests {
    // These imports are EXPECTED to fail in RED phase — the symbols do not
    // exist yet. This file compiles under `cargo check --tests` only, and
    // even then fails with unresolved-import errors. That failure IS the
    // RED evidence.
    use crate::commands::node_commands::{
        add_node, ban_peer, disconnect_peer, get_network_info, get_node_status, list_peers,
        NetworkInfoDto, NodeStatusDto, PeerSummary,
    };

    #[test]
    fn get_node_status_returns_camelcase_dto() {
        // NodeStatusDto must serialize with camelCase keys for JS consumers.
        let dto = NodeStatusDto::default();
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json.get("blockHeight").is_some());
        assert!(json.get("peerCount").is_some());
        assert!(json.get("peerCountIn").is_some());
        assert!(json.get("peerCountOut").is_some());
        assert!(json.get("tipHash").is_some());
        assert!(json.get("headersHeight").is_some());
        assert!(json.get("isSyncing").is_some());
        assert!(json.get("lastBlockTime").is_some());
        assert!(json.get("mempoolSize").is_some());
        assert!(json.get("banCount").is_some());
        assert!(json.get("generatedAt").is_some());
    }

    #[tokio::test]
    async fn get_node_status_command_is_invokable() {
        // Command MUST exist and return a NodeStatusDto (not Result<String,_>).
        let _dto: NodeStatusDto = get_node_status().await.expect("get_node_status");
    }

    #[tokio::test]
    async fn list_peers_returns_peer_summary_vec() {
        let peers: Vec<PeerSummary> = list_peers().await.expect("list_peers");
        let _ = peers;
    }

    #[tokio::test]
    async fn ban_peer_accepts_address_and_reason() {
        ban_peer("192.0.2.1:18351".to_string(), "spam".to_string(), 3600)
            .await
            .expect("ban_peer");
    }

    #[tokio::test]
    async fn disconnect_peer_accepts_address() {
        disconnect_peer("192.0.2.1:18351".to_string())
            .await
            .expect("disconnect_peer");
    }

    #[tokio::test]
    async fn add_node_accepts_host_port() {
        add_node("seed.btpc.org:18351".to_string())
            .await
            .expect("add_node");
    }

    #[tokio::test]
    async fn get_network_info_returns_dto() {
        let _info: NetworkInfoDto = get_network_info().await.expect("get_network_info");
    }

    #[test]
    fn peer_summary_has_required_fields() {
        let summary = PeerSummary::default();
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("address").is_some());
        assert!(json.get("direction").is_some()); // "inbound" | "outbound"
        assert!(json.get("userAgent").is_some());
        assert!(json.get("protocolVersion").is_some());
        assert!(json.get("pingRttMs").is_some());
        assert!(json.get("bytesIn").is_some());
        assert!(json.get("bytesOut").is_some());
        assert!(json.get("banScore").is_some());
        assert!(json.get("connectedSince").is_some());
    }
}
