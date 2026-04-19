/**
 * btpc-node-status.js — Shared Node Status Module (US2, T039-T042)
 *
 * Provides a single observable for node status that every page subscribes to.
 * Replaces per-page polling of get_blockchain_info / get_peer_info with a
 * unified event-driven model.
 *
 * Usage:
 *   <script src="btpc-node-status.js"></script>
 *   <span data-node-status="height"></span>
 *   <span data-node-status="peerCount"></span>
 *
 * Supported data-node-status attributes:
 *   height, tipHash, peerCount, peerCountIn, peerCountOut, syncPct,
 *   network, online, lastBlockTime, mempoolSize, banCount
 */

(function () {
  'use strict';

  // Guard against double-init
  if (window.BTPCNodeStatus && window.BTPCNodeStatus._initialized) {
    return;
  }

  var subscribers = [];
  var unlistenFn = null;
  var offlineTimer = null;
  var OFFLINE_TIMEOUT_MS = 5000; // FR-013: 5 seconds

  // Current state
  var state = {
    blockHeight: 0,
    tipHash: '',
    peerCount: 0,
    peerCountIn: 0,
    peerCountOut: 0,
    syncProgress: 0,
    network: '',
    online: false,
    isSyncing: false,
    lastBlockTime: null,
    mempoolSize: 0,
    banCount: 0,
    generatedAt: 0
  };

  // Field name mapping for data-node-status attributes
  var fieldMap = {
    height: 'blockHeight',
    tipHash: 'tipHash',
    peerCount: 'peerCount',
    peerCountIn: 'peerCountIn',
    peerCountOut: 'peerCountOut',
    syncPct: 'syncProgress',
    network: 'network',
    online: 'online',
    lastBlockTime: 'lastBlockTime',
    mempoolSize: 'mempoolSize',
    banCount: 'banCount'
  };

  function updateState(payload) {
    state.blockHeight = payload.current_height || payload.blockHeight || 0;
    state.tipHash = payload.tip_hash || payload.tipHash || '';
    state.peerCount = payload.connected_peers || payload.peerCount || 0;
    state.peerCountIn = payload.peer_count_in || payload.peerCountIn || 0;
    state.peerCountOut = payload.peer_count_out || payload.peerCountOut || 0;
    state.syncProgress = payload.syncProgress || 0;
    state.network = payload.network || '';
    state.isSyncing = payload.is_syncing || payload.isSyncing || false;
    state.lastBlockTime = payload.last_block_time || payload.lastBlockTime || null;
    state.mempoolSize = payload.mempool_size || payload.mempoolSize || 0;
    state.banCount = payload.ban_count || payload.banCount || 0;
    state.generatedAt = payload.generated_at || payload.generatedAt || 0;
    state.online = true;

    // Compute sync percentage if not provided directly
    if (!state.syncProgress && payload.target_height && payload.target_height > 0) {
      state.syncProgress = Math.min(1, payload.current_height / payload.target_height);
    }

    resetOfflineTimer();
    notifySubscribers();
    bindDOM();
  }

  function notifySubscribers() {
    for (var i = 0; i < subscribers.length; i++) {
      try {
        subscribers[i](Object.assign({}, state));
      } catch (e) {
        console.error('[NodeStatus] subscriber error:', e);
      }
    }
  }

  function formatValue(field, value) {
    if (field === 'online') return value ? 'Online' : 'Offline';
    if (field === 'syncPct') return (value * 100).toFixed(1) + '%';
    if (field === 'lastBlockTime' && value) {
      return new Date(value * 1000).toLocaleTimeString();
    }
    if (value === null || value === undefined) return '--';
    return String(value);
  }

  function bindDOM() {
    var elements = document.querySelectorAll('[data-node-status]');
    for (var i = 0; i < elements.length; i++) {
      var el = elements[i];
      var attr = el.getAttribute('data-node-status');
      var stateKey = fieldMap[attr] || attr;
      var value = state[stateKey];
      el.textContent = formatValue(attr, value);

      // Toggle offline class on the element
      if (attr === 'online') {
        el.classList.toggle('btpc-status-online', state.online);
        el.classList.toggle('btpc-status-offline', !state.online);
      }
    }
  }

  function resetOfflineTimer() {
    if (offlineTimer) clearTimeout(offlineTimer);
    offlineTimer = setTimeout(function () {
      state.online = false;
      notifySubscribers();
      bindDOM();
    }, OFFLINE_TIMEOUT_MS);
  }

  // T041: Wire Tauri event listener for blockchain-event
  function startListening() {
    if (typeof window.tauriListen !== 'function') {
      console.warn('[NodeStatus] tauriListen not available, event stream disabled');
      return;
    }
    window.tauriListen('blockchain-event', function (event) {
      var data = event.payload || event;
      // Route SyncProgressUpdated variant
      if (data.event === 'SyncProgressUpdated' && data.payload) {
        updateState(data.payload);
      } else if (data.current_height !== undefined) {
        // Direct payload (no wrapper)
        updateState(data);
      }
    }).then(function (fn) {
      unlistenFn = fn;
    });
  }

  // T042: Fallback initial fetch via get_shared_node_status command
  function fetchInitialStatus() {
    if (typeof window.tauriInvoke !== 'function') return;
    window.tauriInvoke('get_shared_node_status')
      .then(function (dto) {
        if (dto) updateState(dto);
      })
      .catch(function (err) {
        console.warn('[NodeStatus] initial fetch failed:', err);
      });
  }

  // T051: Cleanup on navigation
  function setupCleanup() {
    window.addEventListener('beforeunload', function () {
      if (offlineTimer) clearTimeout(offlineTimer);
      if (typeof unlistenFn === 'function') {
        unlistenFn();
        unlistenFn = null;
      }
    });
  }

  // Public API
  window.BTPCNodeStatus = {
    _initialized: false,

    /** Idempotent init — safe to call multiple times. */
    init: function () {
      if (this._initialized) return;
      this._initialized = true;
      startListening();
      fetchInitialStatus();
      setupCleanup();
      bindDOM();
    },

    /** Subscribe to state changes. Returns unsubscribe function. */
    subscribe: function (handler) {
      subscribers.push(handler);
      // Immediately fire with current state
      try { handler(Object.assign({}, state)); } catch (_) { /* ignore */ }
      return function () {
        var idx = subscribers.indexOf(handler);
        if (idx !== -1) subscribers.splice(idx, 1);
      };
    },

    /** Get current state snapshot. */
    getState: function () {
      return Object.assign({}, state);
    }
  };

  // Auto-init when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function () {
      window.BTPCNodeStatus.init();
    });
  } else {
    window.BTPCNodeStatus.init();
  }
})();
