/**
 * BTPC Comprehensive Error Handling Module
 *
 * Provides user-friendly error messages with clear guidance
 * Constitution Compliance: Article XI.4 - Clear Error Messages
 */

/**
 * Error codes and their user-friendly messages
 */
const ERROR_CATALOG = {
    // Node errors
    NODE_START_FAILED: {
        what: 'Failed to start node',
        why: 'The node process could not be initialized',
        action: 'Check if another node is running or if the port is in use'
    },
    NODE_STOP_FAILED: {
        what: 'Failed to stop node',
        why: 'The node process did not respond to termination signal',
        action: 'The node may have already stopped. Check the process manually if needed'
    },

    // Wallet errors
    WALLET_CREATION_FAILED: {
        what: 'Failed to create wallet',
        why: 'The wallet could not be generated or saved',
        action: 'Ensure you have sufficient disk space and proper permissions'
    },
    WALLET_CORRUPTION: {
        what: 'Wallet data corruption detected',
        why: 'The wallet file appears to be damaged or tampered with',
        action: 'Restore from backup or create a new wallet',
        severity: 'critical'
    },
    PASSWORD_TOO_WEAK: {
        what: 'Password does not meet requirements',
        why: 'The password is too simple or short',
        action: 'Use at least 8 characters with a mix of letters, numbers, and symbols'
    },

    // Network errors
    CONNECTION_LOST: {
        what: 'Connection to network lost',
        why: 'Unable to communicate with BTPC network peers',
        action: 'Check your internet connection and firewall settings',
        recoverable: true
    },
    NETWORK_TIMEOUT: {
        what: 'Network request timed out',
        why: 'The server did not respond within the expected time',
        action: 'Check your connection or try again later',
        recoverable: true
    },

    // Transaction errors
    INVALID_TRANSACTION: {
        what: 'Transaction validation failed',
        why: 'The transaction does not meet network requirements',
        action: 'Review transaction details and ensure sufficient balance'
    },
    INSUFFICIENT_BALANCE: {
        what: 'Insufficient balance',
        why: 'Your wallet does not have enough BTPC for this transaction',
        action: 'Check your balance and reduce the transaction amount'
    },

    // Configuration errors
    INVALID_SETTING: {
        what: 'Invalid configuration setting',
        why: 'The provided value does not match the expected format',
        action: 'Please check the setting value and try again'
    },
    PORT_IN_USE: {
        what: 'Port already in use',
        why: 'Another application is using the specified port',
        action: 'Stop the other process or choose a different port'
    }
};

/**
 * User-facing error with proper formatting
 */
class UserFacingError extends Error {
    constructor(options) {
        const errorDef = ERROR_CATALOG[options.code] || {};
        super(options.message || errorDef.what || 'An error occurred');

        this.code = options.code;
        this.severity = options.severity || errorDef.severity || 'error';
        this.technical = options.technical;
        this.context = options.context || {};
        this.recoverable = options.recoverable !== undefined ? options.recoverable : errorDef.recoverable;
        this.errorDef = errorDef;
        this.details = options.details;

        // Always include Constitution reference
        this.constitutionRef = options.constitutionRef || 'Article XI.4';
    }

    /**
     * Format error for user display
     */
    format() {
        const formatted = {
            what: this.errorDef.what || this.message,
            why: this.errorDef.why || 'An unexpected error occurred',
            action: this.errorDef.action || 'Please try again or contact support',
            constitutionRef: this.constitutionRef,
            severity: this.severity
        };

        // Add context-specific details
        if (this.context.port) {
            formatted.why = `Port ${this.context.port} is already in use`;
            formatted.action = `Stop the other process using port ${this.context.port} or configure a different port`;
        }

        // Add multilevel details if provided
        if (this.details) {
            formatted.summary = this.details.summary;
            formatted.detailed = this.details.detailed;
            formatted.steps = this.details.steps;
        }

        return formatted;
    }

    /**
     * Check if error requires user action
     */
    requiresUserAction() {
        return this.severity === 'critical' || this.severity === 'error';
    }

    /**
     * Get recovery strategy for the error
     */
    getRecoveryStrategy() {
        if (!this.recoverable) {
            return {
                canRetry: false,
                fallback: 'Contact support if the issue persists'
            };
        }

        return {
            canRetry: true,
            retryDelay: 1000,
            maxRetries: 3,
            fallback: 'Use offline mode or try again later'
        };
    }
}

/**
 * Main error handler
 */
class ErrorHandler {
    constructor(options = {}) {
        this.isDevelopment = options.isDevelopment || window.location.hostname === 'localhost';
        this.logger = new ErrorLogger();
        this.toastManager = new ToastManager();
    }

    /**
     * Handle an error appropriately
     */
    async handle(error, context = {}) {
        // Log the error
        this.logger.logError({
            code: error.code || 'UNKNOWN',
            message: error.message,
            severity: error.severity || 'error',
            context: { ...context, ...error.context },
            stack: error.stack
        });

        // Determine error type and provide guidance
        let userMessage, suggestion, canRetry = false;

        if (error.type === 'network' || error.message.includes('Network')) {
            userMessage = 'Connection problem detected';
            suggestion = 'Check your internet connection and try again';
            canRetry = true;
        } else if (error.message && (error.message.includes('port') || error.message.includes('Port'))) {
            userMessage = 'Port conflict detected';
            suggestion = 'Stop other processes using the port or configure a different port';
            canRetry = false;
        } else if (error.context === 'wallet_creation') {
            userMessage = 'Password requirements not met';
            suggestion = 'Password must be at least 8 characters with a mix of letters and numbers';
        } else if (error instanceof UserFacingError) {
            const formatted = error.format();
            userMessage = formatted.what;
            suggestion = formatted.action;
            canRetry = error.recoverable;
        } else {
            userMessage = 'An unexpected error occurred';
            suggestion = 'Please try again. If the problem persists, restart the application';
        }

        // Show toast notification (deduplicated)
        this.toastManager.showError(userMessage, {
            action: suggestion,
            canRetry
        });

        return {
            userMessage,
            suggestion,
            canRetry,
            technical: this.isDevelopment ? {
                message: error.message,
                stack: error.stack,
                type: error.constructor.name
            } : undefined
        };
    }

    /**
     * Format error for display
     */
    format(error) {
        if (error instanceof UserFacingError) {
            return error.format();
        }

        const result = {
            userMessage: error.message || 'An error occurred',
            severity: 'error'
        };

        if (this.isDevelopment && error.stack) {
            result.technical = {
                stack: error.stack,
                type: error.constructor.name
            };
        }

        return result;
    }

    /**
     * Attempt to recover from an error
     */
    async attemptRecovery(error, retryFn = null) {
        if (!error.retryable && error.retryable !== undefined) {
            return {
                recovered: false,
                fallbackUsed: true,
                userGuidance: 'Please contact support for assistance'
            };
        }

        let attempts = 0;
        const maxAttempts = 3;

        while (attempts < maxAttempts && retryFn) {
            attempts++;
            try {
                const result = await retryFn();
                if (result && result.success) {
                    return {
                        recovered: true,
                        attempts
                    };
                }
            } catch (retryError) {
                console.log(`Recovery attempt ${attempts} failed:`, retryError);
            }

            // Wait before retry
            await new Promise(resolve => setTimeout(resolve, 1000 * attempts));
        }

        return {
            recovered: false,
            attempts,
            fallbackUsed: true,
            userGuidance: 'Please contact support if the issue continues'
        };
    }
}

/**
 * Toast notification manager with deduplication
 */
class ToastManager {
    constructor(options = {}) {
        this.activeToasts = new Map();
        this.messageHistory = new Map();
        this.throttleMs = options.throttleMs || 500;
        this.lastShownTime = 0;
        this.nextId = 1;
        this.groupThreshold = 3;
        this.errorGroup = [];
    }

    /**
     * Show a toast notification
     */
    show(options) {
        const { message, type = 'info' } = options;

        // Check for duplicate
        if (this.messageHistory.has(message)) {
            const lastShown = this.messageHistory.get(message);
            if (Date.now() - lastShown < 5000) {
                console.log('Duplicate toast prevented:', message);
                return null;
            }
        }

        // Throttle rapid notifications
        const now = Date.now();
        if (now - this.lastShownTime < this.throttleMs) {
            this.errorGroup.push(options);
            if (this.errorGroup.length >= this.groupThreshold) {
                return this.showGroupedError();
            }
            return null;
        }

        const id = this.nextId++;
        this.activeToasts.set(id, { message, type, timestamp: now });
        this.messageHistory.set(message, now);
        this.lastShownTime = now;

        // Call actual toast display
        if (window.showToast) {
            window.showToast(options);
        }

        return id;
    }

    /**
     * Show grouped error message
     */
    showGroupedError() {
        const count = this.errorGroup.length;
        const message = `Multiple errors occurred (${count} total)`;

        const id = this.nextId++;
        this.activeToasts.set(id, {
            message,
            type: 'error',
            count,
            timestamp: Date.now()
        });

        if (window.showToast) {
            window.showToast({ message, type: 'error', count });
        }

        this.errorGroup = [];
        return id;
    }

    /**
     * Show error toast
     */
    showError(message, options = {}) {
        return this.show({
            message,
            type: 'error',
            ...options
        });
    }

    /**
     * Hide a toast
     */
    hide(id) {
        this.activeToasts.delete(id);
        if (window.hideToast) {
            window.hideToast(id);
        }
    }

    /**
     * Check if a toast is active
     */
    isActive(id) {
        return this.activeToasts.has(id);
    }
}

/**
 * Error logger for tracking patterns
 */
class ErrorLogger {
    constructor() {
        this.errorHistory = [];
        this.errorPatterns = new Map();
        this.maxHistory = 100;
        this.alertThreshold = 5;
    }

    /**
     * Log an error
     */
    logError(errorInfo) {
        const { message, context, code } = errorInfo;
        const severity = errorInfo.severity || 'error';

        // Add to history
        this.errorHistory.unshift({
            ...errorInfo,
            severity, // Ensure severity is always defined
            timestamp: Date.now()
        });

        // Trim history
        if (this.errorHistory.length > this.maxHistory) {
            this.errorHistory = this.errorHistory.slice(0, this.maxHistory);
        }

        // Track patterns
        const count = (this.errorPatterns.get(code) || 0) + 1;
        this.errorPatterns.set(code, count);

        // Log to console with appropriate level
        const prefix = `[${severity.toUpperCase()}]`;
        if (severity === 'critical') {
            console.error(`${prefix} ${message}`, context);
        } else if (severity === 'warning') {
            console.warn(`${prefix} ${message}`, context);
        } else {
            console.error(`${prefix} ${message}`, context);
        }
    }

    /**
     * Get error patterns
     */
    getErrorPatterns() {
        const patterns = {};
        for (const [code, count] of this.errorPatterns) {
            patterns[code] = count;
        }
        return patterns;
    }

    /**
     * Check if should alert for error pattern
     */
    shouldAlert(code) {
        return (this.errorPatterns.get(code) || 0) >= this.alertThreshold;
    }

    /**
     * Get recent errors
     */
    getRecentErrors(limit = 10) {
        return this.errorHistory.slice(0, limit);
    }
}

// Initialize global error handler
let globalErrorHandler = null;

/**
 * Get or create global error handler
 */
function getErrorHandler() {
    if (!globalErrorHandler) {
        globalErrorHandler = new ErrorHandler();
    }
    return globalErrorHandler;
}

// Set up global error handling
window.addEventListener('error', (event) => {
    const handler = getErrorHandler();
    handler.handle(event.error, { source: 'window.error' });
});

window.addEventListener('unhandledrejection', (event) => {
    const handler = getErrorHandler();
    handler.handle(event.reason, { source: 'unhandledrejection' });
});

// ===========================================================================
// Progressive Disclosure Error Display (T035)
// ===========================================================================

/**
 * Display error with progressive disclosure
 * @param {Object} errorState - Error state from backend
 * @param {string} errorState.error_type - Type of error
 * @param {string} errorState.user_message - User-friendly error message
 * @param {string} [errorState.technical_details] - Technical details (optional)
 */
async function displayError(errorState) {
    const display = document.getElementById('errorDisplay');
    const title = document.getElementById('errorTitle');
    const userMsg = document.getElementById('errorUserMessage');
    const techDetails = document.getElementById('errorTechnicalDetails');
    const detailsSection = document.querySelector('.error-details');

    if (!display || !title || !userMsg || !techDetails) {
        console.error('Error display elements not found in DOM');
        return;
    }

    title.textContent = errorState.error_type;
    userMsg.textContent = errorState.user_message;

    if (errorState.technical_details) {
        techDetails.textContent = errorState.technical_details;
        if (detailsSection) {
            detailsSection.style.display = 'block';
        }
    } else {
        // Hide details section if no technical info
        if (detailsSection) {
            detailsSection.style.display = 'none';
        }
    }

    display.style.display = 'block';
}

/**
 * Initialize progressive disclosure error handler
 * Sets up clipboard copy button and Tauri event listener
 */
function initProgressiveDisclosureErrorHandler() {
    // Copy to clipboard using Tauri API
    const copyButton = document.getElementById('copyErrorButton');
    if (copyButton) {
        copyButton.addEventListener('click', async () => {
            const techDetails = document.getElementById('errorTechnicalDetails');
            if (!techDetails) {
                console.error('Technical details element not found');
                return;
            }

            try {
                // Use Tauri clipboard API
                if (window.__TAURI__ && window.__TAURI__.clipboard) {
                    await window.__TAURI__.clipboard.writeText(techDetails.textContent);
                } else {
                    // Fallback to browser clipboard API
                    await navigator.clipboard.writeText(techDetails.textContent);
                }

                // Show success feedback
                const originalText = copyButton.textContent;
                copyButton.textContent = 'Copied!';
                setTimeout(() => {
                    copyButton.textContent = originalText;
                }, 2000);
            } catch (err) {
                console.error('Failed to copy to clipboard:', err);
                // Show error feedback
                const originalText = copyButton.textContent;
                copyButton.textContent = 'Copy Failed';
                setTimeout(() => {
                    copyButton.textContent = originalText;
                }, 2000);
            }
        });
    }

    // Listen for error events from backend
    if (window.__TAURI__ && window.__TAURI__.event) {
        window.__TAURI__.event.listen('error_occurred', (event) => {
            console.log('Error event received:', event.payload);
            displayError(event.payload);
        });
    } else {
        console.warn('Tauri event API not available - error handling may be limited');
    }
}

// Initialize progressive disclosure on DOMContentLoaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initProgressiveDisclosureErrorHandler);
} else {
    // DOM already loaded
    initProgressiveDisclosureErrorHandler();
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        UserFacingError,
        ErrorHandler,
        ToastManager,
        ErrorLogger,
        getErrorHandler,
        ERROR_CATALOG,
        displayError,
        initProgressiveDisclosureErrorHandler
    };
}

// Make available globally
window.btpcErrorHandler = {
    UserFacingError,
    ErrorHandler,
    ToastManager,
    ErrorLogger,
    getErrorHandler,
    ERROR_CATALOG,
    displayError,
    initProgressiveDisclosureErrorHandler
};