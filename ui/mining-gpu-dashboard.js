/**
 * GPU Mining Dashboard Module - HiveOS Style
 *
 * Features:
 * - Mini GPU stat boxes at top (temp/load%/hashrate)
 * - Cyan (#22d3ee) colored model names
 * - Horizontal temperature progress bars
 * - Mini sparkline graphs for utilization
 * - Dark theme matching HiveOS aesthetic
 */

const { invoke } = window.__TAURI__.core;

class GpuDashboardManager {
    constructor() {
        this.updateInterval = null;
        this.isInitialized = false;
        this.miningStartTime = null;
        // Store historical data for sparklines (last 10 readings per GPU)
        this.utilizationHistory = {};
        // Cache dashboard data for access from mining console
        this.cachedDevices = [];
        this.cachedHealth = {};
        this.cachedStats = {};
        // FIX 2025-12-11: Cache mining status from updateManager (single source of truth)
        // Instead of calling get_mining_status directly (duplicate API call)
        this.cachedMiningStatus = null;
        this.unsubscribeUpdateManager = null;
    }

    async init() {
        if (this.isInitialized) return;
        console.log('[GPU Dashboard] Initializing HiveOS-style display...');

        this.setupEventListeners();

        // FIX 2025-12-11: Subscribe to updateManager for mining status (single source of truth)
        // This eliminates duplicate get_mining_status calls
        if (window.btpcUpdateManager) {
            this.unsubscribeUpdateManager = window.btpcUpdateManager.subscribe((type, data) => {
                if (type === 'mining') {
                    this.cachedMiningStatus = data;
                }
            });
            // Initialize with current state
            this.cachedMiningStatus = window.btpcUpdateManager.getState().mining;
            console.log('[GPU Dashboard] Subscribed to updateManager for mining status (single source of truth)');
        }

        await this.loadDashboardData();

        // Refresh every 2 seconds for responsive updates (GPU data only)
        this.updateInterval = setInterval(() => this.loadDashboardData(), 2000);

        this.isInitialized = true;
        console.log('[GPU Dashboard] Initialized');
    }

    setupEventListeners() {
        const saveThresholdBtn = document.getElementById('save-temp-threshold-btn');
        if (saveThresholdBtn) {
            saveThresholdBtn.addEventListener('click', () => this.saveTemperatureThreshold());
        }
    }

    async loadDashboardData() {
        try {
            const dashboardData = await invoke('get_gpu_dashboard_data');
            // FIX 2025-12-11: Use cached mining status from updateManager (single source of truth)
            // Previously called get_mining_status directly - duplicate API call
            const miningStatus = this.cachedMiningStatus;

            // Cache data for access from mining console
            this.cachedDevices = dashboardData.devices || [];
            this.cachedHealth = dashboardData.health || {};
            this.cachedStats = dashboardData.stats || {};

            this.updateSummaryStats(dashboardData, miningStatus);
            this.renderMiniStatBoxes(dashboardData);
            this.updateTemperatureThreshold(dashboardData.temperature_threshold);
            this.renderGpuRows(dashboardData);

        } catch (error) {
            console.error('[GPU Dashboard] Failed to load data:', error);
            this.showError('Failed to load GPU data');
        }
    }

    updateSummaryStats(dashboardData, miningStatus) {
        let totalHashrate = 0;
        let totalBlocks = 0;

        if (dashboardData.stats) {
            Object.values(dashboardData.stats).forEach(stat => {
                totalHashrate += stat.current_hashrate || 0;
                totalBlocks += stat.lifetime_blocks_found || 0;
            });
        }

        // Total hashrate in cyan
        const totalHashrateEl = document.getElementById('gpu-total-hashrate');
        if (totalHashrateEl) {
            totalHashrateEl.textContent = this.formatHashrate(totalHashrate);
        }

        // GPU count
        const gpuCountEl = document.getElementById('gpu-count');
        if (gpuCountEl) {
            gpuCountEl.textContent = dashboardData.devices ? dashboardData.devices.length : 0;
        }

        // Blocks found (Accepted)
        const acceptedEl = document.getElementById('gpu-accepted');
        if (acceptedEl) acceptedEl.textContent = totalBlocks;

        // Efficiency (always 100% unless we track rejects)
        const efficiencyEl = document.getElementById('gpu-efficiency');
        if (efficiencyEl) {
            efficiencyEl.textContent = totalBlocks > 0 ? '100%' : '--';
        }

        // OpenCL status
        const openclStatusEl = document.getElementById('opencl-status');
        if (openclStatusEl) {
            if (dashboardData.devices && dashboardData.devices.length > 0) {
                openclStatusEl.textContent = 'Ready';
                openclStatusEl.style.color = '#4ade80';
            } else {
                openclStatusEl.textContent = 'No GPUs';
                openclStatusEl.style.color = '#6b7280';
            }
        }

        // Uptime - use backend uptime_seconds (persists across page navigation)
        const uptimeEl = document.getElementById('gpu-uptime');
        if (uptimeEl) {
            const uptimeSeconds = miningStatus?.uptime_seconds || 0;
            if (miningStatus && miningStatus.is_mining && uptimeSeconds > 0) {
                uptimeEl.textContent = this.formatUptime(uptimeSeconds);
            } else {
                uptimeEl.textContent = '0h 0m';
            }
        }
    }

    /**
     * Render mini stat boxes at top (like HiveOS header)
     * Each box shows: temp° / load% / hashrate
     */
    renderMiniStatBoxes(dashboardData) {
        const container = document.getElementById('gpu-mini-stats');
        if (!container) return;

        if (!dashboardData.devices || dashboardData.devices.length === 0) {
            container.innerHTML = '<span style="color: #6b7280; font-size: 0.8rem;">No GPUs detected</span>';
            return;
        }

        let html = '';

        for (const device of dashboardData.devices) {
            const stats = dashboardData.stats[device.device_index] || null;
            const health = dashboardData.health[device.device_index] || null;

            const temp = health?.temperature !== null && health?.temperature !== undefined
                ? health.temperature.toFixed(0) : '--';
            const load = stats && stats.current_hashrate > 0 ? 'ON' : 'OFF';
            const loadColor = load === 'ON' ? '#4ade80' : '#6b7280';
            const hashrate = stats ? this.formatHashrateShort(stats.current_hashrate) : '0';
            const tempColor = this.getTemperatureColor(health?.temperature, dashboardData.temperature_threshold);

            // Mini box like HiveOS: temp° | load% | hashrate
            html += `
                <div style="background: #1a1d21; border: 1px solid #2d3339; border-radius: 4px; padding: 6px 10px; min-width: 90px;">
                    <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
                        <span style="color: ${tempColor}; font-family: 'SF Mono', monospace; font-size: 0.8rem; font-weight: 600;">${temp}°</span>
                        <span style="color: ${loadColor}; font-family: 'SF Mono', monospace; font-size: 0.75rem;">${load}</span>
                        <span style="color: #e5e7eb; font-family: 'SF Mono', monospace; font-size: 0.8rem;">${hashrate}</span>
                    </div>
                </div>
            `;
        }

        container.innerHTML = html;
    }

    updateTemperatureThreshold(threshold) {
        const thresholdInput = document.getElementById('temp-threshold-input');
        if (thresholdInput && document.activeElement !== thresholdInput) {
            thresholdInput.value = Math.round(threshold);
        }
    }

    async saveTemperatureThreshold() {
        const thresholdInput = document.getElementById('temp-threshold-input');
        if (!thresholdInput) return;

        const threshold = parseFloat(thresholdInput.value);

        // Clamp value to valid range
        const clampedThreshold = Math.max(60, Math.min(95, threshold));

        try {
            const savedThreshold = await invoke('set_temperature_threshold', { threshold: clampedThreshold });
            // Visual feedback - briefly highlight the input
            thresholdInput.style.borderColor = '#4ade80';
            setTimeout(() => { thresholdInput.style.borderColor = '#374151'; }, 1000);
            console.log(`[GPU Dashboard] Throttle threshold saved: ${savedThreshold.toFixed(1)}°C`);
        } catch (error) {
            // Visual feedback - briefly highlight red on error
            thresholdInput.style.borderColor = '#f87171';
            setTimeout(() => { thresholdInput.style.borderColor = '#374151'; }, 1000);
            console.error('[GPU Dashboard] Failed to save threshold:', error);
        }
    }

    /**
     * Render GPU rows in HiveOS style
     */
    renderGpuRows(dashboardData) {
        const container = document.getElementById('gpu-devices-container');
        if (!container) return;

        if (!dashboardData.devices || dashboardData.devices.length === 0) {
            container.innerHTML = `
                <div style="text-align: center; padding: 40px 20px; color: #6b7280;">
                    No GPU devices detected. Ensure OpenCL drivers are installed.
                </div>
            `;
            return;
        }

        let html = '';

        for (const device of dashboardData.devices) {
            const stats = dashboardData.stats[device.device_index] || null;
            const health = dashboardData.health[device.device_index] || null;

            const temp = health?.temperature;
            const tempColor = this.getTemperatureColor(temp, dashboardData.temperature_threshold);
            const tempPercent = temp ? Math.min((temp / 100) * 100, 100) : 0;

            const fanSpeed = health?.fan_speed || 0;
            const power = health?.power_consumption || 0;  // Fixed: was power_usage
            const coreClock = health?.core_clock_speed || 0;
            const memClock = health?.memory_clock_speed || 0;
            const hashrate = stats?.current_hashrate || 0;

            // Update utilization history for sparklines
            if (!this.utilizationHistory[device.device_index]) {
                this.utilizationHistory[device.device_index] = [];
            }
            this.utilizationHistory[device.device_index].push(fanSpeed);
            if (this.utilizationHistory[device.device_index].length > 10) {
                this.utilizationHistory[device.device_index].shift();
            }

            // Sparkline SVG for utilization
            const sparklineSvg = this.renderSparkline(this.utilizationHistory[device.device_index], '#22d3ee');

            // Memory info from device
            const memoryMB = device.memory_bytes ? Math.round(device.memory_bytes / 1048576) : 0;

            // Get current threshold value
            const threshold = dashboardData.temperature_threshold || 85.0;

            html += `
                <div style="display: grid; grid-template-columns: 70px 1fr 90px 50px 130px 60px 60px 50px; gap: 8px; padding: 12px 16px; border-bottom: 1px solid #2d3339; align-items: center;">
                    <!-- GPU Index & Hashrate -->
                    <div>
                        <div style="font-weight: 700; font-size: 0.9rem; color: #e5e7eb;">GPU ${device.device_index}</div>
                        <div style="font-size: 0.7rem; color: #6b7280; font-family: 'SF Mono', monospace;">${this.formatHashrateShort(hashrate)}</div>
                    </div>

                    <!-- Model Name (Cyan) & Info -->
                    <div style="overflow: hidden;">
                        <div style="color: #22d3ee; font-weight: 600; font-size: 0.85rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                            ${this.getShortModelName(device.model_name)}
                        </div>
                        <div style="font-size: 0.7rem; color: #6b7280; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                            ${memoryMB > 0 ? memoryMB + ' MiB' : ''} ${device.compute_capability || ''}
                        </div>
                    </div>

                    <!-- Temperature Bar -->
                    <div style="display: flex; align-items: center; gap: 4px;">
                        <div style="flex: 1; height: 8px; background: #1f2937; border-radius: 2px; overflow: hidden;">
                            <div style="width: ${tempPercent}%; height: 100%; background: linear-gradient(90deg, #22d3ee 0%, ${tempColor} 100%); transition: width 0.3s;"></div>
                        </div>
                        <span style="font-size: 0.8rem; font-weight: 600; color: ${tempColor}; font-family: 'SF Mono', monospace; min-width: 28px;">${temp !== null && temp !== undefined ? temp.toFixed(0) + '°' : '--'}</span>
                    </div>

                    <!-- Fan Speed -->
                    <div style="text-align: center; font-family: 'SF Mono', monospace; font-size: 0.85rem; color: #9ca3af;">
                        ${fanSpeed}%
                    </div>

                    <!-- Throttle Threshold Input (larger) -->
                    <div style="display: flex; align-items: center; gap: 4px; justify-content: center;">
                        <input type="number" id="temp-threshold-input" value="${threshold.toFixed(0)}" min="60" max="95" step="1"
                            style="width: 48px; padding: 4px 6px; background: #1f2937; border: 1px solid #374151; border-radius: 4px; color: #e5e7eb; font-family: 'SF Mono', monospace; font-size: 0.85rem; text-align: center;">
                        <span style="font-size: 0.8rem; color: #6b7280;">°C</span>
                        <button onclick="window.gpuDashboard.saveTemperatureThreshold()"
                            style="padding: 4px 10px; background: #374151; border: 1px solid #4b5563; border-radius: 4px; color: #e5e7eb; font-size: 0.8rem; cursor: pointer; font-family: 'SF Mono', monospace;">Set</button>
                    </div>

                    <!-- Core Clock -->
                    <div style="text-align: center; font-family: 'SF Mono', monospace; font-size: 0.8rem; color: #9ca3af;">
                        ${coreClock > 0 ? coreClock : '--'}
                    </div>

                    <!-- Mem Clock -->
                    <div style="text-align: center; font-family: 'SF Mono', monospace; font-size: 0.8rem; color: #9ca3af;">
                        ${memClock > 0 ? memClock : '--'}
                    </div>

                    <!-- Power -->
                    <div style="text-align: center; font-family: 'SF Mono', monospace; font-size: 0.8rem; color: #fbbf24;">
                        ${power > 0 ? power + 'W' : '--'}
                    </div>
                </div>
            `;
        }

        // Add throttle description at bottom after all GPU rows
        html += `
            <div style="padding: 8px 16px; background: #1a1d21; border-top: 1px solid #2d3339; font-size: 0.7rem; color: #6b7280;">
                <span style="color: #9ca3af;">THROTTLE:</span> Mining automatically pauses when GPU temperature exceeds the set threshold to prevent overheating.
            </div>
        `;

        container.innerHTML = html;
    }

    /**
     * Render SVG sparkline graph (like HiveOS mini charts)
     */
    renderSparkline(data, color) {
        if (!data || data.length < 2) {
            return `<svg width="60" height="20" style="opacity: 0.3;"><rect width="60" height="20" fill="#1f2937" rx="2"/></svg>`;
        }

        const width = 60;
        const height = 20;
        const padding = 2;
        const maxVal = Math.max(...data, 100);
        const minVal = Math.min(...data, 0);
        const range = maxVal - minVal || 1;

        // Build SVG path
        const points = data.map((val, i) => {
            const x = padding + (i / (data.length - 1)) * (width - 2 * padding);
            const y = height - padding - ((val - minVal) / range) * (height - 2 * padding);
            return `${x},${y}`;
        });

        const pathD = `M ${points.join(' L ')}`;

        // Area fill path
        const areaD = `M ${padding},${height - padding} L ${points.join(' L ')} L ${width - padding},${height - padding} Z`;

        return `
            <svg width="${width}" height="${height}" style="background: #1f2937; border-radius: 2px;">
                <defs>
                    <linearGradient id="sparkGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                        <stop offset="0%" style="stop-color:${color};stop-opacity:0.4"/>
                        <stop offset="100%" style="stop-color:${color};stop-opacity:0.1"/>
                    </linearGradient>
                </defs>
                <path d="${areaD}" fill="url(#sparkGrad)"/>
                <path d="${pathD}" fill="none" stroke="${color}" stroke-width="1.5"/>
            </svg>
        `;
    }

    /**
     * Get shortened model name for display
     */
    getShortModelName(fullName) {
        if (!fullName) return 'Unknown';

        // Common patterns to shorten
        const replacements = [
            [/NVIDIA GeForce /i, ''],
            [/AMD Radeon /i, ''],
            [/Intel\(R\) /i, ''],
            [/Graphics /i, ''],
        ];

        let name = fullName;
        for (const [pattern, replacement] of replacements) {
            name = name.replace(pattern, replacement);
        }

        // Limit length
        if (name.length > 25) {
            name = name.substring(0, 22) + '...';
        }

        return name;
    }

    getTemperatureColor(temp, threshold) {
        if (temp === null || temp === undefined) return '#6b7280';

        if (temp >= threshold) {
            return '#f87171';  // Red - danger
        } else if (temp >= threshold - 10) {
            return '#fbbf24';  // Yellow - warning
        } else {
            return '#4ade80';  // Green - safe
        }
    }

    formatHashrate(hashrate) {
        if (hashrate >= 1000000000) return (hashrate / 1000000000).toFixed(2) + ' GH/s';
        if (hashrate >= 1000000) return (hashrate / 1000000).toFixed(2) + ' MH/s';
        if (hashrate >= 1000) return (hashrate / 1000).toFixed(2) + ' KH/s';
        return hashrate.toFixed(0) + ' H/s';
    }

    formatHashrateShort(hashrate) {
        if (hashrate >= 1000000000) return (hashrate / 1000000000).toFixed(1) + 'G';
        if (hashrate >= 1000000) return (hashrate / 1000000).toFixed(1) + 'M';
        if (hashrate >= 1000) return (hashrate / 1000).toFixed(1) + 'K';
        return hashrate.toFixed(0);
    }

    formatUptime(seconds) {
        if (!seconds) return '0h 0m';
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);

        if (days > 0) return `${days}d ${hours}h`;
        if (hours > 0) return `${hours}h ${minutes}m`;
        return `${minutes}m ${seconds % 60}s`;
    }

    showError(message) {
        const container = document.getElementById('gpu-devices-container');
        if (container) {
            container.innerHTML = `<div style="text-align: center; padding: 40px 20px; color: #f87171;">${message}</div>`;
        }
    }

    destroy() {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
            this.updateInterval = null;
        }
        // FIX 2025-12-11: Clean up updateManager subscription
        if (this.unsubscribeUpdateManager) {
            this.unsubscribeUpdateManager();
            this.unsubscribeUpdateManager = null;
        }
        this.cachedMiningStatus = null;
        this.isInitialized = false;
    }
}

// Export global instance
window.gpuDashboard = new GpuDashboardManager();

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => window.gpuDashboard.init());
} else {
    window.gpuDashboard.init();
}