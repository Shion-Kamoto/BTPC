/**
 * Comprehensive Error Handling Tests
 * Constitution Compliance: Article XI.4 - Clear Error Messages
 * TDD: Tests written FIRST before implementation
 */

// Import modules under test
const {
    UserFacingError,
    ErrorHandler,
    ToastManager,
    ErrorLogger
} = require('../ui/btpc-error-handler.js');

describe('Comprehensive Error Handling', () => {
    let errorHandler;
    let mockToast;

    beforeEach(() => {
        // Reset error handler
        errorHandler = null;

        // Mock toast notifications
        mockToast = {
            show: jest.fn(),
            hide: jest.fn(),
            count: 0
        };

        window.showToast = mockToast.show;
        window.hideToast = mockToast.hide;
    });

    describe('Error Message Structure', () => {
        test('should include what happened, why, and what to do', () => {
            const error = new UserFacingError({
                code: 'NODE_START_FAILED',
                technical: 'EADDRINUSE: Port 18350 already in use',
                context: { port: 18350 }
            });

            const formatted = error.format();

            // Must explain WHAT happened
            expect(formatted.what).toContain('Failed to start node');

            // Must explain WHY it happened
            expect(formatted.why).toContain('Port 18350 is already in use');

            // Must explain WHAT TO DO
            expect(formatted.action).toContain('Stop the other process');

            // Must include Constitution reference
            expect(formatted.constitutionRef).toBe('Article XI.4');
        });

        test('should categorize errors appropriately', () => {
            const criticalError = new UserFacingError({
                code: 'WALLET_CORRUPTION',
                severity: 'critical'
            });

            const warningError = new UserFacingError({
                code: 'SLOW_NETWORK',
                severity: 'warning'
            });

            expect(criticalError.severity).toBe('critical');
            expect(warningError.severity).toBe('warning');
            expect(criticalError.requiresUserAction()).toBe(true);
            expect(warningError.requiresUserAction()).toBe(false);
        });

        test('should provide recovery strategies', () => {
            const error = new UserFacingError({
                code: 'CONNECTION_LOST',
                recoverable: true
            });

            const recovery = error.getRecoveryStrategy();

            expect(recovery).toBeDefined();
            expect(recovery.canRetry).toBe(true);
            expect(recovery.retryDelay).toBe(1000);
            expect(recovery.maxRetries).toBe(3);
            expect(recovery.fallback).toBeDefined();
        });
    });

    describe('Error Handler Implementation', () => {
        test('should handle different error types appropriately', async () => {
            errorHandler = new ErrorHandler();

            // Network error
            const networkError = new Error('Network request failed');
            networkError.type = 'network';

            const result = await errorHandler.handle(networkError);

            expect(result.userMessage).toContain('Connection problem');
            expect(result.suggestion).toContain('Check your internet');
            expect(result.canRetry).toBe(true);
        });

        test('should provide context-specific guidance', async () => {
            errorHandler = new ErrorHandler();

            // Wallet creation error
            const walletError = new Error('Password too weak');
            walletError.context = 'wallet_creation';

            const result = await errorHandler.handle(walletError);

            expect(result.userMessage).toContain('Password requirements');
            expect(result.suggestion).toContain('at least 8 characters');
            expect(result.suggestion).toContain('mix of letters and numbers');
        });

        test('should include technical details in development mode', () => {
            errorHandler = new ErrorHandler({ isDevelopment: true });

            // Create an actual TypeError to test error type detection
            const error = new TypeError('Cannot read property x of undefined');
            error.stack = 'at function foo (file.js:123)';

            const result = errorHandler.format(error);

            expect(result.technical).toBeDefined();
            expect(result.technical.stack).toContain('file.js:123');
            expect(result.technical.type).toBe('TypeError');
        });

        test('should hide technical details in production', () => {
            errorHandler = new ErrorHandler({ isDevelopment: false });

            const error = new Error('Internal server error');
            error.stack = 'sensitive stack trace';

            const result = errorHandler.format(error);

            expect(result.technical).toBeUndefined();
            expect(result.userMessage).not.toContain('stack trace');
        });
    });

    describe('Toast Notification Deduplication', () => {
        test('should prevent duplicate toast notifications', () => {
            const toastManager = new ToastManager();

            // Show same error multiple times
            toastManager.showError('Connection failed');
            toastManager.showError('Connection failed');
            toastManager.showError('Connection failed');

            // Should only show once
            expect(mockToast.show).toHaveBeenCalledTimes(1);
        });

        test('should track active toasts by ID', () => {
            // Use negative throttleMs to disable throttling (0 would become 500 due to || operator)
            const toastManager = new ToastManager({ throttleMs: -1 });

            const id1 = toastManager.show({ message: 'Error 1', type: 'error' });
            const id2 = toastManager.show({ message: 'Error 2', type: 'error' });

            expect(toastManager.isActive(id1)).toBe(true);
            expect(toastManager.isActive(id2)).toBe(true);

            toastManager.hide(id1);
            expect(toastManager.isActive(id1)).toBe(false);
            expect(toastManager.isActive(id2)).toBe(true);
        });

        test('should prevent rapid-fire notifications', () => {
            const toastManager = new ToastManager({ throttleMs: 500 });

            // Rapid notifications
            for (let i = 0; i < 10; i++) {
                toastManager.showError(`Error ${i}`);
            }

            // Should be throttled
            expect(mockToast.show.mock.calls.length).toBeLessThan(10);
        });

        test('should group similar errors', () => {
            const toastManager = new ToastManager({ throttleMs: 100 });

            // First error shows immediately (not throttled)
            toastManager.showError('Failed to save setting: network error');

            // Rapid errors within throttle window get grouped
            // Need to trigger 3 throttled errors to reach groupThreshold of 3
            toastManager.showError('Failed to save setting: timeout');
            toastManager.showError('Failed to save setting: server error');
            toastManager.showError('Failed to save setting: connection lost');

            // Should group the throttled errors (3 throttled errors)
            const lastCall = mockToast.show.mock.calls[mockToast.show.mock.calls.length - 1];
            expect(lastCall[0].message).toContain('Multiple errors');
            expect(lastCall[0].count).toBe(3);
        });
    });

    describe('Error Logging and Reporting', () => {
        test('should log errors with appropriate levels', () => {
            const logger = new ErrorLogger();
            const consoleSpy = jest.spyOn(console, 'error').mockImplementation();

            logger.logError({
                severity: 'critical',
                message: 'Database corruption detected',
                context: { table: 'wallets' }
            });

            expect(consoleSpy).toHaveBeenCalledWith(
                expect.stringContaining('[CRITICAL]'),
                expect.objectContaining({ table: 'wallets' })
            );

            consoleSpy.mockRestore();
        });

        test('should track error frequency for patterns', () => {
            const logger = new ErrorLogger();

            // Log similar errors
            for (let i = 0; i < 5; i++) {
                logger.logError({
                    code: 'NETWORK_TIMEOUT',
                    message: 'Request timed out'
                });
            }

            const patterns = logger.getErrorPatterns();
            expect(patterns['NETWORK_TIMEOUT']).toBe(5);
            expect(logger.shouldAlert('NETWORK_TIMEOUT')).toBe(true);
        });

        test('should provide error history for debugging', () => {
            const logger = new ErrorLogger();

            logger.logError({ code: 'ERROR_1', timestamp: Date.now() });
            logger.logError({ code: 'ERROR_2', timestamp: Date.now() });

            const history = logger.getRecentErrors(5);
            expect(history.length).toBe(2);
            // unshift() puts newest first, so ERROR_2 (logged second) is at index 0
            expect(history[0].code).toBe('ERROR_2');
        });
    });

    describe('Constitution Compliance', () => {
        test('should comply with Article XI.4 - Clear error messages', () => {
            const error = new UserFacingError({
                code: 'INVALID_TRANSACTION'
            });

            const formatted = error.format();

            // Must have all required components
            expect(formatted.what).toBeDefined();
            expect(formatted.why).toBeDefined();
            expect(formatted.action).toBeDefined();
            expect(formatted.constitutionRef).toContain('Article XI');
        });

        test('should provide multilevel error details', () => {
            const error = new UserFacingError({
                code: 'COMPLEX_ERROR',
                details: {
                    summary: 'Quick explanation',
                    detailed: 'Comprehensive technical explanation',
                    steps: ['Step 1', 'Step 2', 'Step 3']
                }
            });

            const formatted = error.format();
            expect(formatted.summary).toBeDefined();
            expect(formatted.detailed).toBeDefined();
            expect(formatted.steps).toHaveLength(3);
        });
    });

    describe('Error Recovery Actions', () => {
        test('should provide automatic retry for transient errors', async () => {
            const errorHandler = new ErrorHandler();
            const retryable = new Error('Network timeout');
            retryable.code = 'ETIMEDOUT';
            retryable.retryable = true;

            const recovery = await errorHandler.attemptRecovery(retryable, async () => {
                // Simulate successful retry
                return { success: true };
            });

            expect(recovery.recovered).toBe(true);
            expect(recovery.attempts).toBeGreaterThan(0);
        });

        test('should fallback gracefully when recovery fails', async () => {
            const errorHandler = new ErrorHandler();
            const unrecoverable = new Error('Critical failure');
            unrecoverable.retryable = false;

            const recovery = await errorHandler.attemptRecovery(unrecoverable);

            expect(recovery.recovered).toBe(false);
            expect(recovery.fallbackUsed).toBe(true);
            expect(recovery.userGuidance).toContain('Please contact support');
        });
    });
});

// Custom Jest matchers
expect.extend({
    toHaveUserFriendlyMessage(error) {
        const formatted = error.format ? error.format() : error;

        const hasWhat = formatted.what && formatted.what.length > 0;
        const hasWhy = formatted.why && formatted.why.length > 0;
        const hasAction = formatted.action && formatted.action.length > 0;

        const pass = hasWhat && hasWhy && hasAction;

        return {
            pass,
            message: () => pass
                ? `Error has user-friendly message with all components`
                : `Error missing: ${!hasWhat ? 'what' : ''} ${!hasWhy ? 'why' : ''} ${!hasAction ? 'action' : ''}`
        };
    },

    toBeConstitutionCompliant(error) {
        const formatted = error.format ? error.format() : error;
        const hasRef = formatted.constitutionRef && formatted.constitutionRef.includes('Article');
        const hasGuidance = formatted.action && formatted.action.length > 10;

        const pass = hasRef && hasGuidance;

        return {
            pass,
            message: () => pass
                ? `Error is Constitution compliant`
                : `Error not Constitution compliant: ${!hasRef ? 'missing Article reference' : 'insufficient guidance'}`
        };
    }
});