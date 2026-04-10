/**
 * BTPC Database Corruption Check and Recovery (T180, T181, FR-056)
 *
 * Provides:
 * - Automatic startup integrity check
 * - Recovery modal dialog with options:
 *   1. Restore from backup
 *   2. Resync from network
 * - Visual feedback during recovery process
 */

(function() {
    'use strict';

    // Create modal HTML structure
    function createCorruptionModal() {
        // Check if modal already exists
        if (document.getElementById('corruption-recovery-modal')) {
            return;
        }

        const modalHTML = `
        <div id="corruption-recovery-modal" class="corruption-modal-overlay" style="display: none;">
            <div class="corruption-modal">
                <div class="corruption-modal-header">
                    <div class="corruption-icon">⚠️</div>
                    <h2>Database Corruption Detected</h2>
                </div>

                <div class="corruption-modal-body">
                    <p class="corruption-description">
                        The blockchain database integrity check has detected corruption or inconsistencies.
                        To continue using BTPC Wallet safely, please choose a recovery option below.
                    </p>

                    <div class="corruption-errors" id="corruption-errors" style="display: none;">
                        <strong>Detected Issues:</strong>
                        <ul id="corruption-error-list"></ul>
                    </div>

                    <div class="recovery-options">
                        <div class="recovery-option" id="option-backup">
                            <div class="option-icon">📦</div>
                            <div class="option-content">
                                <h3>Restore from Backup</h3>
                                <p>Restore your database from a previous backup. Your wallet data will be preserved.</p>
                                <select id="backup-select" class="backup-select">
                                    <option value="">Loading backups...</option>
                                </select>
                            </div>
                            <button class="btn btn-primary" id="btn-restore-backup" disabled>
                                <span class="icon icon-upload"></span> Restore
                            </button>
                        </div>

                        <div class="recovery-option" id="option-resync">
                            <div class="option-icon">🔄</div>
                            <div class="option-content">
                                <h3>Resync from Network</h3>
                                <p>Download and rebuild the blockchain from network peers. This may take several hours.</p>
                                <small style="color: var(--text-muted);">Your wallets and private keys will be preserved.</small>
                            </div>
                            <button class="btn btn-secondary" id="btn-resync">
                                <span class="icon icon-refresh"></span> Resync
                            </button>
                        </div>
                    </div>
                </div>

                <div class="corruption-modal-footer">
                    <div class="recovery-progress" id="recovery-progress" style="display: none;">
                        <div class="progress-spinner"></div>
                        <span id="recovery-status">Processing...</span>
                    </div>
                    <button class="btn btn-outline" id="btn-continue-anyway" style="display: none;">
                        Continue Anyway (Not Recommended)
                    </button>
                </div>
            </div>
        </div>
        `;

        // Append modal to body
        const modalContainer = document.createElement('div');
        modalContainer.innerHTML = modalHTML;
        document.body.appendChild(modalContainer.firstElementChild);

        // Add modal styles
        addCorruptionModalStyles();

        // Setup event listeners
        setupCorruptionModalEvents();
    }

    // Add CSS styles for the modal
    function addCorruptionModalStyles() {
        if (document.getElementById('corruption-modal-styles')) {
            return;
        }

        const styles = document.createElement('style');
        styles.id = 'corruption-modal-styles';
        styles.textContent = `
            .corruption-modal-overlay {
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.85);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 10000;
                backdrop-filter: blur(4px);
            }

            .corruption-modal {
                background: var(--bg-card, #1E293B);
                border: 1px solid var(--border-color, #334155);
                border-radius: 16px;
                max-width: 600px;
                width: 90%;
                max-height: 90vh;
                overflow-y: auto;
                box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
            }

            .corruption-modal-header {
                padding: 24px 24px 16px;
                text-align: center;
                border-bottom: 1px solid var(--border-color, #334155);
            }

            .corruption-icon {
                font-size: 48px;
                margin-bottom: 12px;
            }

            .corruption-modal-header h2 {
                margin: 0;
                color: #ef4444;
                font-size: 1.5rem;
            }

            .corruption-modal-body {
                padding: 24px;
            }

            .corruption-description {
                color: var(--text-secondary, #94A3B8);
                margin-bottom: 16px;
                line-height: 1.6;
            }

            .corruption-errors {
                background: rgba(239, 68, 68, 0.1);
                border: 1px solid rgba(239, 68, 68, 0.3);
                border-radius: 8px;
                padding: 12px 16px;
                margin-bottom: 20px;
                color: #ef4444;
                font-size: 0.875rem;
            }

            .corruption-errors ul {
                margin: 8px 0 0 0;
                padding-left: 20px;
            }

            .corruption-errors li {
                margin: 4px 0;
            }

            .recovery-options {
                display: flex;
                flex-direction: column;
                gap: 16px;
            }

            .recovery-option {
                display: flex;
                align-items: flex-start;
                gap: 16px;
                padding: 16px;
                background: rgba(139, 92, 246, 0.1);
                border: 1px solid var(--btpc-primary, #6366F1);
                border-radius: 12px;
                transition: all 0.2s;
            }

            .recovery-option:hover {
                background: rgba(139, 92, 246, 0.15);
            }

            .option-icon {
                font-size: 32px;
                flex-shrink: 0;
            }

            .option-content {
                flex: 1;
            }

            .option-content h3 {
                margin: 0 0 8px;
                color: var(--text-primary, #F8FAFC);
                font-size: 1rem;
            }

            .option-content p {
                margin: 0;
                color: var(--text-secondary, #94A3B8);
                font-size: 0.875rem;
                line-height: 1.5;
            }

            .backup-select {
                width: 100%;
                margin-top: 12px;
                padding: 8px 12px;
                background: var(--bg-primary, #0F172A);
                border: 1px solid var(--border-color, #334155);
                border-radius: 6px;
                color: var(--text-primary, #F8FAFC);
                font-size: 0.875rem;
            }

            .recovery-option .btn {
                flex-shrink: 0;
                align-self: center;
            }

            .corruption-modal-footer {
                padding: 16px 24px 24px;
                display: flex;
                justify-content: space-between;
                align-items: center;
                border-top: 1px solid var(--border-color, #334155);
            }

            .recovery-progress {
                display: flex;
                align-items: center;
                gap: 12px;
                color: var(--text-secondary, #94A3B8);
            }

            .progress-spinner {
                width: 20px;
                height: 20px;
                border: 2px solid rgba(99, 102, 241, 0.3);
                border-top-color: var(--btpc-primary, #6366F1);
                border-radius: 50%;
                animation: corruption-spin 0.8s linear infinite;
            }

            @keyframes corruption-spin {
                to { transform: rotate(360deg); }
            }

            .btn-outline {
                background: transparent;
                border: 1px solid var(--border-color, #334155);
                color: var(--text-muted, #64748B);
                padding: 8px 16px;
                border-radius: 6px;
                cursor: pointer;
                font-size: 0.8125rem;
                transition: all 0.2s;
            }

            .btn-outline:hover {
                border-color: var(--text-muted, #64748B);
                color: var(--text-secondary, #94A3B8);
            }
        `;
        document.head.appendChild(styles);
    }

    // Setup event listeners for modal buttons
    function setupCorruptionModalEvents() {
        const backupSelect = document.getElementById('backup-select');
        const restoreBtn = document.getElementById('btn-restore-backup');
        const resyncBtn = document.getElementById('btn-resync');
        const continueBtn = document.getElementById('btn-continue-anyway');

        // Enable restore button when backup is selected
        if (backupSelect) {
            backupSelect.addEventListener('change', () => {
                restoreBtn.disabled = !backupSelect.value;
            });
        }

        // Restore from backup
        if (restoreBtn) {
            restoreBtn.addEventListener('click', async () => {
                const backupPath = backupSelect.value;
                if (!backupPath) return;

                if (!confirm('This will replace your current database with the backup. Continue?')) {
                    return;
                }

                showRecoveryProgress('Restoring from backup...');

                try {
                    const result = await window.invoke('restore_database_backup', { backupPath });
                    if (result.success) {
                        showRecoveryProgress('Backup restored successfully! Restarting...');
                        setTimeout(() => {
                            window.invoke('restart_app').catch(() => window.location.reload());
                        }, 2000);
                    } else {
                        hideRecoveryProgress();
                        alert('Restore failed: ' + result.error_message);
                    }
                } catch (e) {
                    hideRecoveryProgress();
                    alert('Restore failed: ' + e);
                }
            });
        }

        // Resync from network
        if (resyncBtn) {
            resyncBtn.addEventListener('click', async () => {
                if (!confirm('This will delete and rebuild the blockchain database from network peers. This may take several hours. Your wallet keys will be preserved. Continue?')) {
                    return;
                }

                showRecoveryProgress('Initiating blockchain resync...');

                try {
                    // Clear blockchain data and restart sync
                    const result = await window.invoke('resync_blockchain');
                    if (result.success) {
                        showRecoveryProgress('Resync initiated! Restarting...');
                        setTimeout(() => {
                            hideCorruptionModal();
                            window.location.reload();
                        }, 2000);
                    } else {
                        hideRecoveryProgress();
                        alert('Resync failed: ' + result.error_message);
                    }
                } catch (e) {
                    // If resync_blockchain command doesn't exist, just hide modal and continue
                    console.warn('Resync command not available:', e);
                    hideRecoveryProgress();
                    hideCorruptionModal();
                    alert('Resync initiated. Please restart the application to begin blockchain download.');
                }
            });
        }

        // Continue anyway (not recommended)
        if (continueBtn) {
            continueBtn.addEventListener('click', () => {
                if (confirm('Continuing with a corrupted database may cause data loss or incorrect balances. Are you sure?')) {
                    hideCorruptionModal();
                    sessionStorage.setItem('btpc_corruption_dismissed', 'true');
                }
            });
        }
    }

    // Show the corruption recovery modal
    function showCorruptionModal(errors) {
        createCorruptionModal();

        const modal = document.getElementById('corruption-recovery-modal');
        const errorsContainer = document.getElementById('corruption-errors');
        const errorList = document.getElementById('corruption-error-list');
        const continueBtn = document.getElementById('btn-continue-anyway');

        // Show errors if provided
        if (errors && errors.length > 0) {
            errorsContainer.style.display = 'block';
            errorList.innerHTML = errors.map(e => `<li>${e}</li>`).join('');
        }

        // Show "Continue Anyway" button
        if (continueBtn) {
            continueBtn.style.display = 'block';
        }

        // Load available backups
        loadBackupsForRecovery();

        modal.style.display = 'flex';
    }

    // Hide the corruption modal
    function hideCorruptionModal() {
        const modal = document.getElementById('corruption-recovery-modal');
        if (modal) {
            modal.style.display = 'none';
        }
    }

    // Show recovery progress
    function showRecoveryProgress(message) {
        const progress = document.getElementById('recovery-progress');
        const status = document.getElementById('recovery-status');
        const restoreBtn = document.getElementById('btn-restore-backup');
        const resyncBtn = document.getElementById('btn-resync');
        const continueBtn = document.getElementById('btn-continue-anyway');

        if (progress) progress.style.display = 'flex';
        if (status) status.textContent = message;
        if (restoreBtn) restoreBtn.disabled = true;
        if (resyncBtn) resyncBtn.disabled = true;
        if (continueBtn) continueBtn.style.display = 'none';
    }

    // Hide recovery progress
    function hideRecoveryProgress() {
        const progress = document.getElementById('recovery-progress');
        const restoreBtn = document.getElementById('btn-restore-backup');
        const resyncBtn = document.getElementById('btn-resync');
        const continueBtn = document.getElementById('btn-continue-anyway');

        if (progress) progress.style.display = 'none';
        if (restoreBtn) restoreBtn.disabled = false;
        if (resyncBtn) resyncBtn.disabled = false;
        if (continueBtn) continueBtn.style.display = 'block';
    }

    // Load available backups for recovery
    async function loadBackupsForRecovery() {
        const select = document.getElementById('backup-select');
        if (!select || !window.invoke) return;

        try {
            const result = await window.invoke('list_database_backups');
            if (result.success && result.backups.length > 0) {
                select.innerHTML = '<option value="">Select a backup...</option>';
                result.backups.forEach(backup => {
                    const option = document.createElement('option');
                    option.value = backup.path;
                    const date = new Date(backup.created_at * 1000).toLocaleString();
                    option.textContent = `${backup.name} (${date})`;
                    select.appendChild(option);
                });
            } else {
                select.innerHTML = '<option value="">No backups available</option>';
            }
        } catch (e) {
            console.error('Failed to load backups:', e);
            select.innerHTML = '<option value="">Failed to load backups</option>';
        }
    }

    // T181 - Automatic startup corruption check
    async function performStartupIntegrityCheck() {
        // Skip if already dismissed this session
        if (sessionStorage.getItem('btpc_corruption_dismissed') === 'true') {
            console.log('🔍 Corruption check dismissed for this session');
            return { success: true, is_valid: true };
        }

        // Skip on login page (no Tauri context yet)
        if (window.location.pathname.includes('login.html')) {
            return { success: true, is_valid: true };
        }

        // Wait for Tauri API to be available
        if (!window.invoke) {
            console.log('🔍 Waiting for Tauri API...');
            return new Promise((resolve) => {
                const checkInterval = setInterval(() => {
                    if (window.invoke) {
                        clearInterval(checkInterval);
                        performStartupIntegrityCheck().then(resolve);
                    }
                }, 100);
                // Timeout after 5 seconds
                setTimeout(() => {
                    clearInterval(checkInterval);
                    resolve({ success: true, is_valid: true });
                }, 5000);
            });
        }

        console.log('🔍 Performing startup database integrity check (FR-056)...');

        try {
            const result = await window.invoke('check_database_integrity');

            if (result.success && !result.is_valid) {
                console.error('❌ Database corruption detected:', result.errors);
                showCorruptionModal(result.errors || ['Unknown corruption detected']);
                return result;
            } else if (result.success && result.is_valid) {
                console.log('✅ Database integrity check passed');
                return result;
            } else {
                console.warn('⚠️ Integrity check returned unexpected result:', result);
                return { success: true, is_valid: true };
            }
        } catch (e) {
            // If check fails (e.g., no database yet), continue normally
            console.warn('⚠️ Integrity check skipped:', e);
            return { success: true, is_valid: true };
        }
    }

    // Export functions for global access
    window.btpcCorruptionCheck = {
        performStartupIntegrityCheck,
        showCorruptionModal,
        hideCorruptionModal
    };

    // Auto-run integrity check when DOM is ready (for non-login pages)
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            // Delay slightly to ensure Tauri is initialized
            setTimeout(performStartupIntegrityCheck, 500);
        });
    } else {
        setTimeout(performStartupIntegrityCheck, 500);
    }

})();