/**
 * BTPC Password Modal - Wallet Encryption UI Integration
 *
 * Handles:
 * - Checking wallet lock status on page load
 * - Unlocking encrypted wallets with password
 * - Auto-detecting migration from plaintext
 * - Error handling and user feedback
 * - Show/hide password toggle
 * - Keyboard shortcuts (Enter to submit, Escape to cancel migration)
 */

// Tauri API is initialized by btpc-tauri-context.js (loaded before this script)
// Use window.invoke which is set up by btpc-tauri-context.js
// No need for immediate Tauri API check - functions will check when called

// Password Modal State
const PasswordModal = {
    // DOM Elements (initialized in init())
    overlay: null,
    input: null,
    toggleBtn: null,
    unlockBtn: null,
    cancelBtn: null,
    errorDiv: null,
    loadingDiv: null,
    migrationNotice: null,
    modalTitle: null,
    modalDescription: null,

    // State
    isMigrationMode: false,
    isUnlocking: false,

    /**
     * Initialize the password modal
     * Call this on DOMContentLoaded
     */
    init() {
        // Get DOM elements
        this.overlay = document.getElementById('password-modal-overlay');
        this.input = document.getElementById('master-password');
        this.toggleBtn = document.getElementById('toggle-password');
        this.unlockBtn = document.getElementById('btn-unlock');
        this.cancelBtn = document.getElementById('btn-cancel');
        this.errorDiv = document.getElementById('password-error');
        this.loadingDiv = document.getElementById('password-loading');
        this.migrationNotice = document.getElementById('migration-notice');
        this.modalTitle = document.getElementById('modal-title');
        this.modalDescription = document.getElementById('modal-description');

        // Validate required elements exist
        if (!this.overlay || !this.input || !this.unlockBtn) {
            console.error('CRITICAL: Required password modal elements not found in DOM');
            throw new Error('Password modal initialization failed: required elements missing');
        }

        // Attach event listeners
        this.attachEventListeners();

        // Check wallet lock status on page load
        this.checkLockStatus();
    },

    /**
     * Attach event listeners to modal elements
     */
    attachEventListeners() {
        // Show/hide password toggle
        this.toggleBtn.addEventListener('click', () => {
            const type = this.input.type === 'password' ? 'text' : 'password';
            this.input.type = type;
            this.toggleBtn.textContent = type === 'password' ? '👁️' : '🙈';
        });

        // Unlock button
        this.unlockBtn.addEventListener('click', () => {
            this.handleUnlock();
        });

        // Cancel button (migration mode only)
        this.cancelBtn.addEventListener('click', () => {
            this.hideModal();
        });

        // Enter key to submit
        this.input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter' && !this.isUnlocking) {
                this.handleUnlock();
            }
        });

        // Escape key to cancel (migration mode only)
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && this.isMigrationMode && !this.isUnlocking) {
                this.hideModal();
            }
        });

        // Auto-focus password input when modal shows
        this.overlay.addEventListener('transitionend', () => {
            if (this.overlay.classList.contains('show')) {
                this.input.focus();
            }
        });
    },

    /**
     * Check wallet lock status and show modal if locked
     */
    async checkLockStatus() {
        // Wait for Tauri API to be ready
        if (!window.invoke) {
            console.log('[PasswordModal] Tauri API not ready, retrying in 500ms...');
            setTimeout(() => this.checkLockStatus(), 500);
            return;
        }

        try {
            const isLocked = await window.invoke('check_wallet_lock_status');

            if (isLocked) {
                // Wallets are locked, show password modal
                this.showModal(false); // Not migration mode
            } else {
                // Wallets already unlocked, hide modal
                this.hideModal();
            }
        } catch (error) {
            console.error('[PasswordModal] Failed to check lock status:', error);

            // Check if error indicates missing encrypted file (migration needed)
            if (error.includes('No such file') || error.includes('not found')) {
                // Show migration mode modal
                this.showModal(true); // Migration mode
            } else {
                // Unknown error, show generic modal
                this.showModal(false);
            }
        }
    },

    /**
     * Show the password modal
     * @param {boolean} migrationMode - Whether to show migration UI
     */
    showModal(migrationMode = false) {
        this.isMigrationMode = migrationMode;

        if (migrationMode) {
            // Migration Mode: Encrypt plaintext wallet metadata
            this.modalTitle.textContent = '⚡ Upgrade to Encrypted Wallets';
            this.modalDescription.textContent = 'Create a master password to encrypt your wallet metadata.';
            this.migrationNotice.classList.add('show');
            this.unlockBtn.textContent = 'Encrypt & Unlock';
            this.cancelBtn.style.display = 'block'; // Show cancel button
        } else {
            // Unlock Mode: Normal password prompt
            this.modalTitle.textContent = '🔒 Unlock Your Wallets';
            this.modalDescription.textContent = 'Enter your master password to access your encrypted wallet metadata.';
            this.migrationNotice.classList.remove('show');
            this.unlockBtn.textContent = 'Unlock Wallets';
            this.cancelBtn.style.display = 'none'; // Hide cancel button
        }

        // Show modal
        this.overlay.classList.add('show');

        // Clear previous state
        this.input.value = '';
        this.input.type = 'password';
        this.toggleBtn.textContent = '👁️';
        this.hideError();
        this.hideLoading();

        // Focus password input
        setTimeout(() => this.input.focus(), 100);
    },

    /**
     * Hide the password modal
     */
    hideModal() {
        this.overlay.classList.remove('show');
        this.input.value = ''; // Clear password from memory
        this.hideError();
        this.hideLoading();
    },

    /**
     * Handle unlock button click
     */
    async handleUnlock() {
        const password = this.input.value.trim();

        // Validate password input
        if (!password) {
            this.showError('Please enter a password');
            return;
        }

        // Prevent double-click
        if (this.isUnlocking) {
            return;
        }

        // Show loading state
        this.isUnlocking = true;
        this.showLoading();
        this.hideError();
        this.unlockBtn.disabled = true;

        try {
            if (this.isMigrationMode) {
                // Migration: Encrypt plaintext wallet metadata
                const result = await window.invoke('migrate_to_encrypted', { password });
                console.log('[PasswordModal] Migration successful:', result);

                // Show success message briefly
                this.showSuccess('Migration complete! Wallets unlocked.');

                // Hide modal after 1.5 seconds
                setTimeout(() => {
                    this.hideModal();
                    // Reload page to refresh wallet data
                    window.location.reload();
                }, 1500);
            } else {
                // Normal unlock: Decrypt encrypted wallet metadata
                const result = await window.invoke('unlock_wallets', { password });
                console.log('[PasswordModal] Unlock successful:', result);

                // Hide modal immediately
                this.hideModal();

                // Reload page to display unlocked wallet data
                window.location.reload();
            }
        } catch (error) {
            console.error('[PasswordModal] Unlock/migration failed:', error);

            // Parse error message
            let errorMsg = 'Unknown error occurred';

            if (typeof error === 'string') {
                if (error.includes('Failed to decrypt') || error.includes('wrong password')) {
                    errorMsg = 'Incorrect password. Please try again.';
                } else if (error.includes('already encrypted')) {
                    errorMsg = 'Wallets are already encrypted. Use unlock instead.';
                    // Switch to unlock mode
                    setTimeout(() => {
                        this.showModal(false);
                    }, 2000);
                } else if (error.includes('No plaintext wallet metadata')) {
                    errorMsg = 'No wallet metadata found. Create a wallet first.';
                } else {
                    errorMsg = error;
                }
            }

            this.showError(errorMsg);
        } finally {
            // Reset loading state
            this.isUnlocking = false;
            this.hideLoading();
            this.unlockBtn.disabled = false;

            // Re-focus input for retry
            this.input.focus();
            this.input.select();
        }
    },

    /**
     * Show error message
     * @param {string} message - Error message to display
     */
    showError(message) {
        this.errorDiv.textContent = message;
        this.errorDiv.classList.add('show');
    },

    /**
     * Hide error message
     */
    hideError() {
        this.errorDiv.textContent = '';
        this.errorDiv.classList.remove('show');
    },

    /**
     * Show success message (reuses error div with different styling)
     * @param {string} message - Success message to display
     */
    showSuccess(message) {
        this.errorDiv.textContent = message;
        this.errorDiv.style.color = '#4ade80'; // Green
        this.errorDiv.style.backgroundColor = 'rgba(74, 222, 128, 0.1)';
        this.errorDiv.style.borderColor = 'rgba(74, 222, 128, 0.3)';
        this.errorDiv.classList.add('show');

        // Reset styling after use
        setTimeout(() => {
            this.errorDiv.style.color = '#ff6b6b'; // Red (default)
            this.errorDiv.style.backgroundColor = 'rgba(255, 107, 107, 0.1)';
            this.errorDiv.style.borderColor = 'rgba(255, 107, 107, 0.3)';
        }, 2000);
    },

    /**
     * Show loading spinner
     */
    showLoading() {
        this.loadingDiv.classList.add('show');
    },

    /**
     * Hide loading spinner
     */
    hideLoading() {
        this.loadingDiv.classList.remove('show');
    }
};

// Global functions for external integration
window.PasswordModal = PasswordModal;

/**
 * Lock wallets and show password modal
 * Called from settings page or lock button
 */
window.lockWallets = async function() {
    try {
        const result = await window.invoke('lock_wallets');
        console.log('[PasswordModal] Wallets locked:', result);

        // Show password modal
        PasswordModal.showModal(false);

        // Optional: Reload page to clear wallet data from UI
        // window.location.reload();
    } catch (error) {
        console.error('[PasswordModal] Failed to lock wallets:', error);
        alert('Failed to lock wallets: ' + error);
    }
};

/**
 * Change master password
 * Called from settings page
 * @param {string} oldPassword - Current master password
 * @param {string} newPassword - New master password
 */
window.changeMasterPassword = async function(oldPassword, newPassword) {
    try {
        const result = await window.invoke('change_master_password', {
            oldPassword,
            newPassword
        });
        console.log('[PasswordModal] Password changed:', result);
        return { success: true, message: result };
    } catch (error) {
        console.error('[PasswordModal] Failed to change password:', error);
        return { success: false, error: error };
    }
};

// Auto-initialize on page load
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        PasswordModal.init();
    });
} else {
    // DOM already loaded
    PasswordModal.init();
}