/**
 * GPU Mining Dashboard Module (Feature 012: T023-T026)
 *
 * Implements GPU mining dashboard with individual GPU cards, real-time stats,
 * health monitoring, and thermal throttling indicators.
 *
 * Article XI Compliance:
 * - Backend-first: All data from Tauri commands
 * - Event-driven: Updates via Tauri events
 * - No localStorage/sessionStorage usage
 * - Single source of truth: Backend AppState
 */

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/**
 * GPU Dashboard Manager
 */
class GpuDashboardManager {
    constructor() {
        this.gpuCards = new Map();  // device_index -> HTMLElement
        this.updateInterval = null;
        this.isInitialized = false;
    }

    /**
     * Initialize GPU dashboard (T023)
     *
     * Sets up event listeners and loads initial data.
     */
    async init() {
        if (this.isInitialized) return;

        console.log('[GPU Dashboard] Initializing...');

        // Set up UI event listeners
        this.setupEventListeners();

        // Set up event-driven updates (Article XI: Backend emits, frontend listens)
        await this.startEventListening();

        // Load initial dashboard data
        await this.loadDashboardData();

        this.isInitialized = true;
        console.log('[GPU Dashboard] Initialized successfully');
    }

    /**
     * Setup UI event listeners (T023)
     */
    setupEventListeners() {
        // Temperature threshold save button
        const saveThresholdBtn = document.getElementById('save-temp-threshold-btn');
        if (saveThresholdBtn) {
            saveThresholdBtn.addEventListener('click', () => this.saveTemperatureThreshold());
        }

        // Temperature threshold input validation
        const thresholdInput = document.getElementById('temp-threshold-input');
        if (thresholdInput) {
            thresholdInput.addEventListener('input', (e) => {
                const value = parseFloat(e.target.value);
                if (value < 60 || value > 95) {
                    e.target.setCustomValidity('Temperature must be between 60°C and 95°C');
                } else {
                    e.target.setCustomValidity('');
                }
            });
        }
    }

    /**
     * Load complete dashboard data from backend (T023)
     *
     * Article XI: Single backend query for all GPU data.
     */
    async loadDashboardData() {
        try {
            console.log('[GPU Dashboard] Loading dashboard data...');

            const dashboardData = await invoke('get_gpu_dashboard_data');

            // Update temperature threshold input
            this.updateTemperatureThreshold(dashboardData.temperature_threshold);

            // Render GPU cards
            this.renderGpuCards(dashboardData);

            console.log('[GPU Dashboard] Data loaded successfully:', dashboardData);
        } catch (error) {
            console.error('[GPU Dashboard] Failed to load data:', error);
            this.showError('Failed to load GPU dashboard data: ' + error);
        }
    }

    /**
     * Update temperature threshold display (T025)
     */
    updateTemperatureThreshold(threshold) {
        const thresholdInput = document.getElementById('temp-threshold-input');
        if (thresholdInput) {
            thresholdInput.value = threshold.toFixed(1);
        }
    }

    /**
     * Save temperature threshold to backend (T025)
     *
     * Article XI: Backend validates FIRST before saving.
     */
    async saveTemperatureThreshold() {
        const thresholdInput = document.getElementById('temp-threshold-input');
        const feedbackDiv = document.getElementById('temp-threshold-feedback');
        const saveBtn = document.getElementById('save-temp-threshold-btn');

        if (!thresholdInput || !feedbackDiv) return;

        const threshold = parseFloat(thresholdInput.value);

        // Disable button during save
        saveBtn.disabled = true;
        feedbackDiv.style.display = 'none';

        try {
            // Backend validates and saves
            const savedThreshold = await invoke('set_temperature_threshold', { threshold });

            // Success feedback
            feedbackDiv.textContent = `✓ Threshold saved: ${savedThreshold.toFixed(1)}°C`;
            feedbackDiv.style.color = '#4ade80';
            feedbackDiv.style.display = 'block';

            console.log('[GPU Dashboard] Threshold saved:', savedThreshold);

            // Hide feedback after 3 seconds
            setTimeout(() => {
                feedbackDiv.style.display = 'none';
            }, 3000);

        } catch (error) {
            // Error feedback
            feedbackDiv.textContent = '✗ ' + error;
            feedbackDiv.style.color = '#f87171';
            feedbackDiv.style.display = 'block';
            console.error('[GPU Dashboard] Failed to save threshold:', error);
        } finally {
            saveBtn.disabled = false;
        }
    }

    /**
     * Render GPU cards from dashboard data (T024)
     *
     * Creates individual GPU card for each device with stats, health, and throttle indicators.
     */
    renderGpuCards(dashboardData) {
        const container = document.getElementById('gpu-cards-container');
        const loadingDiv = document.getElementById('gpu-dashboard-loading');
        const noGpusDiv = document.getElementById('gpu-dashboard-no-gpus');

        if (!container) return;

        // Handle no GPUs case
        if (!dashboardData.devices || dashboardData.devices.length === 0) {
            loadingDiv.style.display = 'none';
            noGpusDiv.style.display = 'block';
            container.style.display = 'none';
            return;
        }

        // Hide loading, show cards
        loadingDiv.style.display = 'none';
        noGpusDiv.style.display = 'none';
        container.style.display = 'grid';

        // Clear existing cards
        container.innerHTML = '';
        this.gpuCards.clear();

        // Create card for each GPU
        for (const device of dashboardData.devices) {
            const card = this.createGpuCard(device, dashboardData);
            container.appendChild(card);
            this.gpuCards.set(device.device_index, card);
        }
    }

    /**
     * Create individual GPU card (T024)
     */
    createGpuCard(device, dashboardData) {
        const card = document.createElement('div');
        card.className = 'gpu-card';
        card.id = `gpu-card-${device.device_index}`;

        const stats = dashboardData.stats[device.device_index] || null;
        const health = dashboardData.health[device.device_index] || null;

        // GPU header
        const header = `
            <div class="gpu-card-header">
                <div class="gpu-icon">🖥️</div>
                <div class="gpu-info">
                    <div class="gpu-name">${this.escapeHtml(device.model_name)}</div>
                    <div class="gpu-index">GPU ${device.device_index}</div>
                </div>
                ${this.renderMiningStatus(stats)}
            </div>
        `;

        // Mining stats section
        const miningStats = stats ? `
            <div class="gpu-section">
                <div class="section-title">⛏️ Mining Statistics</div>
                <div class="stats-grid">
                    <div class="stat-item">
                        <div class="stat-label">Hashrate</div>
                        <div class="stat-value">${this.formatHashrate(stats.current_hashrate)}</div>
                    </div>
                    <div class="stat-item">
                        <div class="stat-label">Blocks Found</div>
                        <div class="stat-value">${stats.lifetime_blocks_found}</div>
                    </div>
                    <div class="stat-item">
                        <div class="stat-label">Mining Uptime</div>
                        <div class="stat-value">${this.formatUptime(stats.mining_uptime)}</div>
                    </div>
                    <div class="stat-item">
                        <div class="stat-label">Mining Intensity</div>
                        <div class="stat-value">${stats.throttle_percentage}%</div>
                    </div>
                </div>
            </div>
        ` : '<div class="gpu-section"><div class="stat-placeholder">Mining not active</div></div>';

        // Health metrics section
        const healthMetrics = health ? `
            <div class="gpu-section">
                <div class="section-title">🌡️ Health Metrics</div>
                <div class="stats-grid">
                    ${this.renderHealthStat('Temperature', health.temperature, '°C', this.getTemperatureColor(health.temperature, dashboardData.temperature_threshold))}
                    ${this.renderHealthStat('Fan Speed', health.fan_speed, 'RPM')}
                    ${this.renderHealthStat('Power', health.power_consumption, 'W')}
                    ${this.renderHealthStat('Memory Used', health.memory_used ? `${health.memory_used}/${health.memory_total}` : null, 'MB')}
                </div>
            </div>
        ` : '<div class="gpu-section"><div class="stat-placeholder">Health sensors unavailable</div></div>';

        // Throttle warning
        const throttleWarning = stats && stats.throttle_percentage < 100 ? `
            <div class="throttle-warning">
                ⚠️ Thermal throttling active (${stats.throttle_percentage}% intensity)
            </div>
        ` : '';

        card.innerHTML = header + miningStats + healthMetrics + throttleWarning;

        return card;
    }

    /**
     * Render mining status badge (T024)
     */
    renderMiningStatus(stats) {
        if (!stats) {
            return '<div class="status-badge status-idle">Idle</div>';
        }

        const statusClass = {
            'Active': 'status-active',
            'Idle': 'status-idle',
            'Error': 'status-error',
            'Throttled': 'status-throttled'
        }[stats.mining_status] || 'status-idle';

        return `<div class="status-badge ${statusClass}">${stats.mining_status}</div>`;
    }

    /**
     * Render health stat item (T024)
     */
    renderHealthStat(label, value, unit, color = null) {
        if (value === null || value === undefined) {
            return `
                <div class="stat-item">
                    <div class="stat-label">${label}</div>
                    <div class="stat-value stat-unavailable">N/A</div>
                </div>
            `;
        }

        const formattedValue = typeof value === 'number' ? value.toFixed(1) : value;
        const colorStyle = color ? `style="color: ${color}"` : '';

        return `
            <div class="stat-item">
                <div class="stat-label">${label}</div>
                <div class="stat-value" ${colorStyle}>${formattedValue}${unit}</div>
            </div>
        `;
    }

    /**
     * Get temperature color based on threshold (T024)
     */
    getTemperatureColor(temp, threshold) {
        if (!temp) return null;

        if (temp >= threshold) {
            return '#f87171';  // Red - above threshold
        } else if (temp >= threshold - 10) {
            return '#fbbf24';  // Yellow - approaching threshold
        } else {
            return '#4ade80';  // Green - safe temperature
        }
    }

    /**
     * Format hashrate for display (T024)
     */
    formatHashrate(hashrate) {
        if (hashrate >= 1000000) {
            return (hashrate / 1000000).toFixed(2) + ' MH/s';
        } else if (hashrate >= 1000) {
            return (hashrate / 1000).toFixed(2) + ' KH/s';
        } else {
            return hashrate.toFixed(2) + ' H/s';
        }
    }

    /**
     * Format uptime for display (T024)
     */
    formatUptime(seconds) {
        if (!seconds) return '0s';

        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;

        if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else if (minutes > 0) {
            return `${minutes}m ${secs}s`;
        } else {
            return `${secs}s`;
        }
    }

    /**
     * Start event-driven updates (T021/T024)
     *
     * Listens to backend events emitted every 5 seconds.
     * Article XI: Backend is single source of truth, frontend listens (no polling).
     */
    async startEventListening() {
        // Load initial data immediately
        await this.loadDashboardData();

        // Listen to gpu-stats-updated events from backend (emitted every 5s)
        this.eventUnlisten = await window.__TAURI__.event.listen('gpu-stats-updated', (event) => {
            console.log('[GPU Dashboard] Received gpu-stats-updated event');
            this.handleGpuStatsUpdate(event.payload);
        });

        console.log('[GPU Dashboard] Started event listening (backend emits every 5s)');
    }

    /**
     * Handle GPU stats update from backend event (T021/T024)
     */
    handleGpuStatsUpdate(data) {
        // Update dashboard with new data from backend
        this.renderGpuCards(data.devices, data.stats, data.health);

        // Update temperature threshold if changed
        if (data.temperature_threshold !== undefined) {
            const thresholdInput = document.getElementById('temperature-threshold');
            if (thresholdInput && thresholdInput.value !== data.temperature_threshold.toString()) {
                thresholdInput.value = data.temperature_threshold;
            }
        }
    }

    /**
     * Stop event listening (T024)
     */
    stopEventListening() {
        if (this.eventUnlisten) {
            this.eventUnlisten();
            this.eventUnlisten = null;
            console.log('[GPU Dashboard] Stopped event listening');
        }
    }

    /**
     * Show error message (T023)
     */
    showError(message) {
        const container = document.getElementById('gpu-cards-container');
        const loadingDiv = document.getElementById('gpu-dashboard-loading');
        const noGpusDiv = document.getElementById('gpu-dashboard-no-gpus');

        if (loadingDiv) loadingDiv.style.display = 'none';
        if (noGpusDiv) noGpusDiv.style.display = 'none';

        if (container) {
            container.innerHTML = `
                <div class="error-message">
                    ⚠️ ${this.escapeHtml(message)}
                </div>
            `;
            container.style.display = 'block';
        }
    }

    /**
     * Escape HTML for security (T023)
     */
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    /**
     * Cleanup (T026)
     */
    destroy() {
        this.stopEventListening();
        this.gpuCards.clear();
        this.isInitialized = false;
    }
}

// Export global instance
window.gpuDashboard = new GpuDashboardManager();

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        window.gpuDashboard.init();
    });
} else {
    window.gpuDashboard.init();
}
