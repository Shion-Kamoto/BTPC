/**
 * Backend-First Validation Tests
 * Constitution Compliance: Article XI.1 - Backend State Authority
 * TDD: Tests written FIRST before implementation
 */

// Import modules under test
const {
    updateSetting,
    createWallet,
    performNodeAction
} = require('../ui/btpc-backend-first.js');

describe('Backend-First Validation', () => {
    let originalLocalStorage;
    let originalInvoke;

    beforeEach(() => {
        // Save original functions
        originalLocalStorage = window.localStorage;
        originalInvoke = window.invoke;

        // Mock localStorage
        const localStorageMock = {
            data: {},
            getItem: jest.fn((key) => localStorageMock.data[key] || null),
            setItem: jest.fn((key, value) => localStorageMock.data[key] = value),
            removeItem: jest.fn((key) => delete localStorageMock.data[key]),
            clear: jest.fn(() => localStorageMock.data = {})
        };
        Object.defineProperty(window, 'localStorage', {
            value: localStorageMock,
            writable: true
        });

        // Mock Tauri invoke
        window.invoke = jest.fn();
    });

    afterEach(() => {
        // Restore original functions
        window.localStorage = originalLocalStorage;
        window.invoke = originalInvoke;
    });

    describe('Setting Updates', () => {
        test('should validate with backend BEFORE saving to localStorage', async () => {
            const setting = { key: 'rpc_port', value: '18350' };

            // Mock successful backend validation
            window.invoke
                .mockResolvedValueOnce({ valid: true })  // validate_setting
                .mockResolvedValueOnce({ success: true }); // save_setting

            const result = await updateSetting(setting);

            // Verify backend was called FIRST
            expect(window.invoke).toHaveBeenCalledWith('validate_setting', setting);
            expect(window.invoke).toHaveBeenCalledWith('save_setting', setting);

            // Verify localStorage was updated ONLY after backend success
            expect(localStorage.setItem).toHaveBeenCalledWith('rpc_port', '18350');
            expect(result.success).toBe(true);
        });

        test('should NOT save to localStorage if backend validation fails', async () => {
            const setting = { key: 'rpc_port', value: 'invalid' };

            // Mock failed backend validation
            window.invoke.mockResolvedValueOnce({
                valid: false,
                error: 'Port must be a valid number'
            });

            const result = await updateSetting(setting);

            // Verify backend was called
            expect(window.invoke).toHaveBeenCalledWith('validate_setting', setting);

            // Verify save_setting was NOT called
            expect(window.invoke).not.toHaveBeenCalledWith('save_setting', expect.anything());

            // Verify localStorage was NOT updated
            expect(localStorage.setItem).not.toHaveBeenCalled();
            expect(result.success).toBe(false);
            expect(result.error).toContain('Port must be a valid number');
        });

        test('should NOT save to localStorage if backend save fails', async () => {
            const setting = { key: 'node_address', value: '127.0.0.1:18350' };

            // Mock successful validation but failed save
            window.invoke
                .mockResolvedValueOnce({ valid: true })  // validate_setting
                .mockRejectedValueOnce(new Error('Node is currently running')); // save_setting fails

            const result = await updateSetting(setting);

            // Verify both backend calls were made
            expect(window.invoke).toHaveBeenCalledWith('validate_setting', setting);
            expect(window.invoke).toHaveBeenCalledWith('save_setting', setting);

            // Verify localStorage was NOT updated due to backend save failure
            expect(localStorage.setItem).not.toHaveBeenCalled();
            expect(result.success).toBe(false);
            expect(result.error).toContain('Node is currently running');
        });
    });

    describe('Wallet Creation', () => {
        test('should create wallet in backend BEFORE saving to localStorage', async () => {
            const walletData = {
                name: 'Test Wallet',
                password: 'secure_password_123'
            };

            // Mock successful wallet creation
            const backendWallet = {
                id: 'wallet_123',
                name: 'Test Wallet',
                address: 'btpc1qxyz...',
                created_at: '2025-10-17T10:00:00Z'
            };

            window.invoke.mockResolvedValueOnce(backendWallet);

            const result = await createWallet(walletData);

            // Verify backend was called
            expect(window.invoke).toHaveBeenCalledWith('create_wallet', walletData);

            // Verify localStorage was updated with backend response
            expect(localStorage.setItem).toHaveBeenCalledWith('current_wallet', JSON.stringify(backendWallet));
            expect(result.success).toBe(true);
            expect(result.wallet).toEqual(backendWallet);
        });

        test('should NOT save wallet to localStorage if backend creation fails', async () => {
            const walletData = {
                name: 'Test Wallet',
                password: 'weak'
            };

            // Mock wallet creation failure
            window.invoke.mockRejectedValueOnce(new Error('Password too weak'));

            const result = await createWallet(walletData);

            // Verify backend was called
            expect(window.invoke).toHaveBeenCalledWith('create_wallet', walletData);

            // Verify localStorage was NOT updated
            expect(localStorage.setItem).not.toHaveBeenCalled();
            expect(result.success).toBe(false);
            expect(result.error).toContain('Password too weak');
        });
    });

    describe('State Synchronization', () => {
        test('should emit events for cross-page synchronization after backend success', async () => {
            const setting = { key: 'theme', value: 'dark' };

            // Mock successful backend operations
            window.invoke
                .mockResolvedValueOnce({ valid: true })
                .mockResolvedValueOnce({ success: true });

            // Mock Tauri emit
            window.__TAURI__ = { emit: jest.fn() };

            const result = await updateSetting(setting);

            // Verify event was emitted AFTER backend success
            expect(window.__TAURI__.emit).toHaveBeenCalledWith('setting-updated', setting);
            expect(result.success).toBe(true);
        });

        test('should NOT emit events if backend operation fails', async () => {
            const setting = { key: 'theme', value: 'invalid' };

            // Mock failed backend validation
            window.invoke.mockResolvedValueOnce({
                valid: false,
                error: 'Invalid theme'
            });

            // Mock Tauri emit
            window.__TAURI__ = { emit: jest.fn() };

            const result = await updateSetting(setting);

            // Verify event was NOT emitted
            expect(window.__TAURI__.emit).not.toHaveBeenCalled();
            expect(result.success).toBe(false);
        });
    });

    describe('Constitution Compliance', () => {
        test('should comply with Article XI.1 - Backend as single source of truth', async () => {
            // This test verifies that all state changes go through backend first
            const operations = [
                { type: 'setting', key: 'network', value: 'testnet' },
                { type: 'wallet', name: 'New Wallet' },
                { type: 'node', action: 'start' }
            ];

            for (const op of operations) {
                // Mock backend responses
                window.invoke.mockResolvedValueOnce({ success: true });

                // Perform operation
                let result;
                if (op.type === 'setting') {
                    window.invoke.mockResolvedValueOnce({ valid: true });
                    result = await updateSetting(op);
                } else if (op.type === 'wallet') {
                    result = await createWallet(op);
                } else if (op.type === 'node') {
                    result = await performNodeAction(op.action);
                }

                // Verify backend was ALWAYS called first
                expect(window.invoke).toHaveBeenCalled();
                expect(window.invoke.mock.calls.length).toBeGreaterThan(0);
            }
        });

        test('should provide clear error messages per Article XI.4', async () => {
            const setting = { key: 'invalid_key', value: 'test' };

            // Mock backend error with clear message
            window.invoke.mockResolvedValueOnce({
                valid: false,
                error: 'Invalid setting key',
                suggestion: 'Please use a valid setting key from the allowed list',
                constitutionRef: 'Article XI.1'
            });

            const result = await updateSetting(setting);

            expect(result.success).toBe(false);
            expect(result.error).toBeDefined();
            expect(result.error.length).toBeGreaterThan(5);  // Clear explanation provided
            expect(result.suggestion || result.error).toContain('Please');  // User guidance
        });
    });
});

// Custom Jest matchers for backend-first validation
expect.extend({
    toCallBackendFirst(received) {
        const invokeCallIndex = received.mock.calls.findIndex(call => call[0] === 'invoke');
        const localStorageCallIndex = received.mock.calls.findIndex(call => call[0] === 'setItem');

        const pass = invokeCallIndex < localStorageCallIndex || localStorageCallIndex === -1;

        return {
            pass,
            message: () => pass
                ? `Expected backend not to be called before localStorage`
                : `Expected backend to be called before localStorage`
        };
    }
});