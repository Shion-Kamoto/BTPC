/**
 * BTPC Update Manager - Centralized state and update coordination
 * Prevents duplicate calls, manages refresh intervals, handles errors gracefully
 */

class BtpcUpdateManager {
    constructor() {
        this.state = {
            node: { is_running: false, block_height: 0, last_updated: null },
            mining: { is_mining: false, hashrate: 0, blocks_found: 0, last_updated: null },
            blockchain: {
                height: 0,
                headers: 0,
                difficulty: 0,
                chain: 'mainnet',
                sync_progress: 0,
                is_synced: false,
                last_updated: null
            },
            wallet: { balance: 0, address_count: 0, last_updated: null },
            transactions: [],
            network: { network: 'regtest', rpc_port: 18360, p2p_port: 18361, rpc_host: '127.0.0.1', last_updated: null },
        };

        this.listeners = [];
        this.updateInProgress = false;
        this.intervals = [];
        this.errorCount = 0;
        this.maxErrors = 5;
        this.isAutoUpdateRunning = false; // Article XI: Singleton guard to prevent duplicate starts
    }

    /**
     * Subscribe to state changes
     */
    subscribe(listener) {
        this.listeners.push(listener);
        // Return unsubscribe function
        return () => {
            this.listeners = this.listeners.filter(l => l !== listener);
        };
    }

    /**
     * Notify all listeners of state change
     */
    notifyListeners(type, data) {
        this.listeners.forEach(listener => {
            try {
                listener(type, data, this.state);
            } catch (e) {
                console.error('Listener error:', e);
            }
        });
    }

    /**
     * Update node status
     */
    async updateNodeStatus() {
        if (!window.invoke) return;

        try {
            const status = await window.invoke('get_node_status');
            const changed = this.state.node.running !== status.running;

            this.state.node = {
                ...status,
                last_updated: Date.now()
            };

            if (changed) {
                console.log('Node status changed:', status.running ? 'RUNNING' : 'STOPPED');
            }

            this.notifyListeners('node', this.state.node);
            this.errorCount = Math.max(0, this.errorCount - 1); // Reduce error count on success
            return this.state.node;
        } catch (e) {
            console.warn('Failed to get node status:', e);
            this.errorCount++;
            return null;
        }
    }

    /**
     * Update mining status
     */
    async updateMiningStatus() {
        if (!window.invoke) return;

        try {
            const status = await window.invoke('get_mining_status');
            const changed = this.state.mining.is_mining !== status.is_mining;

            this.state.mining = {
                ...status,
                last_updated: Date.now()
            };

            if (changed) {
                console.log('Mining status changed:', status.is_mining ? 'ACTIVE' : 'INACTIVE');
            }

            this.notifyListeners('mining', this.state.mining);
            this.errorCount = Math.max(0, this.errorCount - 1);
            return this.state.mining;
        } catch (e) {
            console.warn('Failed to get mining status:', e);
            this.errorCount++;
            return null;
        }
    }

    /**
     * Update blockchain info
     */
    async updateBlockchainInfo() {
        if (!window.invoke) return;

        try {
            const info = await window.invoke('get_blockchain_info');
            const height = info.blocks || info.height || 0;
            const headers = info.headers || height;
            const chain = info.chain || 'mainnet';

            // Calculate sync progress
            const sync_progress = headers > 0 ? Math.min(100, (height / headers) * 100) : 0;
            const is_synced = sync_progress >= 99.9; // Consider synced if >= 99.9%

            const changed = this.state.blockchain.height !== height;

            this.state.blockchain = {
                height: height,
                headers: headers,
                difficulty: info.difficulty || 0,
                chain: chain,
                best_block_hash: info.best_block_hash || info.bestblockhash || '0'.repeat(64),
                connections: info.connections || 0,
                sync_progress: sync_progress,
                is_synced: is_synced,
                last_updated: Date.now()
            };

            if (changed) {
                console.log('Blockchain updated: height', this.state.blockchain.height, '/', this.state.blockchain.headers, `(${sync_progress.toFixed(1)}%)`);
            }

            this.notifyListeners('blockchain', this.state.blockchain);
            this.errorCount = Math.max(0, this.errorCount - 1);
            return this.state.blockchain;
        } catch (e) {
            console.warn('Failed to get blockchain info:', e);
            this.errorCount++;
            return null;
        }
    }

    /**
     * Update wallet balance
     */
    async updateWalletBalance() {
        if (!window.invoke) return;

        try {
            // Refresh wallet balances from UTXO manager first (sync cached balance with actual)
            try {
                await window.invoke('refresh_all_wallet_balances');
            } catch (refreshErr) {
                console.debug('Balance refresh skipped (might be first load):', refreshErr);
            }

            const summary = await window.invoke('get_wallet_summary');

            this.state.wallet = {
                balance: summary.total_balance_btp || 0,
                address_count: summary.total_wallets || 0,
                last_updated: Date.now()
            };

            this.notifyListeners('wallet', this.state.wallet);
            this.errorCount = Math.max(0, this.errorCount - 1);
            return this.state.wallet;
        } catch (e) {
            console.warn('Failed to get wallet balance:', e);
            this.errorCount++;
            return null;
        }
    }

    /**
     * Update network configuration
     */
    async updateNetworkConfig() {
        if (!window.invoke) return;

        try {
            const config = await window.invoke('get_network_config');

            this.state.network = {
                network: config.network || 'regtest',
                rpc_port: config.rpc_port || 18360,
                p2p_port: config.p2p_port || 18361,
                rpc_host: config.rpc_host || '127.0.0.1',
                last_updated: Date.now()
            };

            this.notifyListeners('network', this.state.network);
            this.errorCount = Math.max(0, this.errorCount - 1);
            return this.state.network;
        } catch (e) {
            console.warn('Failed to get network config:', e);
            this.errorCount++;
            return null;
        }
    }

    /**
     * Update all statuses (coordinated, non-overlapping)
     */
    async updateAll() {
        if (this.updateInProgress) {
            console.debug('Update already in progress, skipping...');
            return;
        }

        if (this.errorCount >= this.maxErrors) {
            console.error('Too many errors, pausing updates. Check backend connection.');
            return;
        }

        this.updateInProgress = true;

        try {
            // Run updates in parallel but track completion
            await Promise.allSettled([
                this.updateNodeStatus(),
                this.updateMiningStatus(),
                this.updateBlockchainInfo(),
                this.updateWalletBalance(),
                this.updateNetworkConfig()
            ]);
        } finally {
            this.updateInProgress = false;
        }
    }

    /**
     * Start automatic updates
     * Article XI compliant: Singleton guard prevents duplicate polling mechanisms
     */
    startAutoUpdate(intervalMs = 5000) {
        // Article XI, Section 11.3: Prevent duplicate polling intervals
        if (this.isAutoUpdateRunning) {
            console.debug('Auto-update already running, ignoring duplicate start request (Article XI compliance)');
            return;
        }

        // Clear any existing intervals
        this.stopAutoUpdate();

        // Update immediately
        this.updateAll();

        // Then update periodically
        const interval = setInterval(() => {
            this.updateAll();
        }, intervalMs);

        this.intervals.push(interval);
        this.isAutoUpdateRunning = true;
        console.log(`✅ Auto-update started (${intervalMs}ms interval)`);
    }

    /**
     * Stop all automatic updates
     */
    stopAutoUpdate() {
        this.intervals.forEach(interval => clearInterval(interval));
        this.intervals = [];
        this.isAutoUpdateRunning = false;
        console.log('Auto-update stopped');
    }

    /**
     * Get current state
     */
    getState() {
        return this.state;
    }

    /**
     * Reset error count (call after user action like manual refresh)
     */
    resetErrors() {
        this.errorCount = 0;
    }
}

// Create global instance
window.btpcUpdateManager = window.btpcUpdateManager || new BtpcUpdateManager();
