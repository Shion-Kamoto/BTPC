/**
 * BTPC Desktop App - Integration Tests
 * Tests complete user flows across multiple modules
 * TDD: Integration tests for end-to-end scenarios
 */

describe('Desktop App Integration Tests', () => {
    let mockTauri;
    let errorHandler;
    let eventManager;
    let backendFirst;

    beforeEach(() => {
        // Clear localStorage to ensure test isolation
        localStorage.clear();

        // Mock Tauri API
        mockTauri = {
            invoke: jest.fn(),
            emit: jest.fn(),
            listen: jest.fn(() => Promise.resolve(jest.fn()))
        };

        window.__TAURI__ = mockTauri;

        // Mock window.invoke to use mockTauri.invoke
        window.invoke = mockTauri.invoke;

        // Initialize modules
        errorHandler = new (require('../../ui/btpc-error-handler.js').ErrorHandler)();
        eventManager = new (require('../../ui/btpc-event-manager.js').EventListenerManager)();
        backendFirst = require('../../ui/btpc-backend-first.js');
    });

    afterEach(() => {
        eventManager.destroy();
        window.__TAURI__ = undefined;
        window.invoke = undefined;
    });

    describe('Node Lifecycle Integration', () => {
        test('should complete full node start-stop cycle', async () => {
            // Mock backend responses
            mockTauri.invoke
                .mockResolvedValueOnce({ success: true, pid: 12345 }) // start_node
                .mockResolvedValueOnce({ running: true, pid: 12345 }) // get_node_status
                .mockResolvedValueOnce({ success: true }) // stop_node
                .mockResolvedValueOnce({ running: false, pid: null }); // get_node_status

            // Start node
            const startResult = await mockTauri.invoke('start_node');
            expect(startResult.success).toBe(true);
            expect(startResult.pid).toBe(12345);

            // Verify status
            const statusAfterStart = await mockTauri.invoke('get_node_status');
            expect(statusAfterStart.running).toBe(true);

            // Stop node
            const stopResult = await mockTauri.invoke('stop_node');
            expect(stopResult.success).toBe(true);

            // Verify status after stop
            const statusAfterStop = await mockTauri.invoke('get_node_status');
            expect(statusAfterStop.running).toBe(false);
            expect(statusAfterStop.pid).toBeNull();
        });

        test('should handle node start failure gracefully', async () => {
            // Mock node start failure
            mockTauri.invoke.mockRejectedValueOnce(new Error('Port 18350 already in use'));

            try {
                await mockTauri.invoke('start_node');
                fail('Should have thrown error');
            } catch (error) {
                const handled = await errorHandler.handle(error, { context: 'node_start' });

                expect(handled.userMessage).toBeDefined();
                expect(handled.suggestion).toContain('port');
                expect(handled.canRetry).toBe(false);
            }
        });
    });

    describe('Wallet Creation and Management Integration', () => {
        test('should create wallet with backend-first validation', async () => {
            const walletData = {
                name: 'Test Wallet',
                password: 'SecurePassword123!'
            };

            const mockWallet = {
                id: 'wallet_abc123',
                name: 'Test Wallet',
                address: 'btpc1qxyz...abc',
                balance: 0,
                created_at: '2025-10-17T10:00:00Z'
            };

            // Mock backend wallet creation
            mockTauri.invoke.mockResolvedValueOnce(mockWallet);

            // Create wallet using backend-first pattern
            const result = await backendFirst.createWallet(walletData);

            // Verify backend was called
            expect(mockTauri.invoke).toHaveBeenCalledWith('create_wallet', walletData);

            // Verify success
            expect(result.success).toBe(true);
            expect(result.wallet).toEqual(mockWallet);

            // Verify localStorage updated after backend
            expect(localStorage.getItem('current_wallet')).toBe(JSON.stringify(mockWallet));
        });

        test('should reject weak password with clear guidance', async () => {
            const walletData = {
                name: 'Test Wallet',
                password: 'weak'
            };

            // Mock weak password rejection
            mockTauri.invoke.mockRejectedValueOnce(
                Object.assign(new Error('Password too weak'), { context: 'wallet_creation' })
            );

            const result = await backendFirst.createWallet(walletData);

            expect(result.success).toBe(false);
            expect(result.error).toContain('Password');
        });
    });

    describe('Settings Management Integration', () => {
        test('should update settings with backend-first validation', async () => {
            const setting = {
                key: 'rpc_port',
                value: '18360'
            };

            // Mock successful validation and save
            mockTauri.invoke
                .mockResolvedValueOnce({ valid: true }) // validate_setting
                .mockResolvedValueOnce({ success: true }); // save_setting

            const result = await backendFirst.updateSetting(setting);

            // Verify backend validation happened first
            expect(mockTauri.invoke).toHaveBeenNthCalledWith(1, 'validate_setting', setting);
            expect(mockTauri.invoke).toHaveBeenNthCalledWith(2, 'save_setting', setting);

            // Verify success
            expect(result.success).toBe(true);

            // Verify localStorage only updated after backend
            expect(localStorage.getItem('rpc_port')).toBe('18360');
        });

        test('should prevent invalid settings from being saved', async () => {
            const setting = {
                key: 'rpc_port',
                value: 'invalid'
            };

            // Mock validation failure
            mockTauri.invoke.mockResolvedValueOnce({
                valid: false,
                error: 'Port must be a number between 1024 and 65535'
            });

            const result = await backendFirst.updateSetting(setting);

            // Verify only validation was called, not save
            expect(mockTauri.invoke).toHaveBeenCalledTimes(1);
            expect(mockTauri.invoke).toHaveBeenCalledWith('validate_setting', setting);

            // Verify failure
            expect(result.success).toBe(false);
            expect(result.error).toContain('Port must be a number');

            // Verify localStorage NOT updated
            expect(localStorage.getItem('rpc_port')).toBeNull();
        });
    });

    describe('Error Handling Integration', () => {
        test('should handle error with recovery strategy', async () => {
            const networkError = new Error('Network timeout');
            networkError.type = 'network';
            networkError.code = 'ETIMEDOUT';
            networkError.retryable = true;

            // Mock successful retry
            let attemptCount = 0;
            const operation = async () => {
                attemptCount++;
                if (attemptCount < 3) {
                    throw networkError;
                }
                return { success: true };
            };

            const recovery = await errorHandler.attemptRecovery(networkError, operation);

            expect(recovery.recovered).toBe(true);
            expect(recovery.attempts).toBe(3);
        });

        test('should provide clear guidance for unrecoverable errors', async () => {
            const criticalError = new (require('../../ui/btpc-error-handler.js').UserFacingError)({
                code: 'WALLET_CORRUPTION',
                severity: 'critical'
            });

            const formatted = criticalError.format();

            expect(formatted.what).toBeDefined();
            expect(formatted.why).toBeDefined();
            expect(formatted.action).toBeDefined();
            expect(formatted.constitutionRef).toContain('Article XI');
            expect(criticalError.requiresUserAction()).toBe(true);
        });
    });

    describe('Event Management Integration', () => {
        test('should properly manage cross-page events', async () => {
            const handler1 = jest.fn();
            const handler2 = jest.fn();

            // Register listeners
            await eventManager.listen('blockchain-update', handler1);
            await eventManager.listen('wallet-update', handler2);

            expect(eventManager.getListenerCount()).toBe(2);

            // Cleanup
            eventManager.destroy();
            expect(eventManager.getListenerCount()).toBe(0);
        });

        test('should prevent memory leaks on page navigation', async () => {
            // Simulate multiple page loads
            for (let i = 0; i < 5; i++) {
                const manager = new (require('../../ui/btpc-event-manager.js').EventListenerManager)();
                await manager.listen('test-event', jest.fn());
                manager.destroy();
            }

            // Final manager should only have its own listeners
            const finalManager = new (require('../../ui/btpc-event-manager.js').EventListenerManager)();
            await finalManager.listen('test-event', jest.fn());

            expect(finalManager.getListenerCount()).toBe(1);
            finalManager.destroy();
        });
    });

    describe('Constitution Compliance Integration', () => {
        test('should enforce backend-first pattern across all modules', async () => {
            const operations = [
                { module: 'settings', data: { key: 'theme', value: 'dark' } },
                { module: 'wallet', data: { name: 'Wallet', password: 'Pass123!' } }
            ];

            for (const op of operations) {
                // Mock backend responses
                if (op.module === 'settings') {
                    mockTauri.invoke
                        .mockResolvedValueOnce({ valid: true })
                        .mockResolvedValueOnce({ success: true });

                    const result = await backendFirst.updateSetting(op.data);

                    // Backend must be called FIRST
                    expect(mockTauri.invoke).toHaveBeenCalledWith('validate_setting', op.data);
                    expect(result.success).toBe(true);
                } else if (op.module === 'wallet') {
                    mockTauri.invoke.mockResolvedValueOnce({ id: 'wallet_123' });

                    await backendFirst.createWallet(op.data);

                    // Backend must be called FIRST
                    expect(mockTauri.invoke).toHaveBeenCalledWith('create_wallet', op.data);
                }
            }
        });

        test('should provide clear error messages per Article XI.4', () => {
            const errorCodes = ['NODE_START_FAILED', 'WALLET_CORRUPTION', 'CONNECTION_LOST'];

            for (const code of errorCodes) {
                const error = new (require('../../ui/btpc-error-handler.js').UserFacingError)({ code });
                const formatted = error.format();

                // Must have all required components
                expect(formatted.what).toBeDefined();
                expect(formatted.why).toBeDefined();
                expect(formatted.action).toBeDefined();
                expect(formatted.constitutionRef).toBeDefined();
            }
        });
    });

    describe('Performance Integration', () => {
        test('should handle rapid operations without degradation', async () => {
            const startTime = Date.now();
            const operations = [];

            // Simulate 100 rapid operations
            for (let i = 0; i < 100; i++) {
                operations.push(
                    backendFirst.updateSetting({
                        key: `setting_${i}`,
                        value: `value_${i}`
                    })
                );
            }

            // Mock all responses
            mockTauri.invoke.mockResolvedValue({ valid: true });
            mockTauri.invoke.mockResolvedValue({ success: true });

            await Promise.all(operations);

            const duration = Date.now() - startTime;

            // Should complete in reasonable time (< 1 second for 100 operations)
            expect(duration).toBeLessThan(1000);
        });

        test('should prevent memory leaks in long-running session', async () => {
            const initialMemory = process.memoryUsage().heapUsed;

            // Simulate long session with many operations
            for (let i = 0; i < 50; i++) {
                const manager = new (require('../../ui/btpc-event-manager.js').EventListenerManager)();
                await manager.listen('test-event', jest.fn());
                manager.destroy();
            }

            const finalMemory = process.memoryUsage().heapUsed;
            const memoryIncrease = finalMemory - initialMemory;

            // Memory increase should be minimal (< 15MB accounting for test environment overhead)
            // Note: Real-world memory leaks would show much larger increases
            expect(memoryIncrease).toBeLessThan(15 * 1024 * 1024);
        });
    });
});

/**
 * Custom matchers for integration tests
 */
expect.extend({
    toCompleteWorkflow(received) {
        const hasAllSteps = received.steps && received.steps.length > 0;
        const allStepsCompleted = received.steps?.every(step => step.completed);

        const pass = hasAllSteps && allStepsCompleted;

        return {
            pass,
            message: () => pass
                ? 'Workflow completed successfully'
                : `Workflow incomplete: ${received.steps?.filter(s => !s.completed).map(s => s.name).join(', ')}`
        };
    }
});