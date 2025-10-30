/**
 * Tests for Tauri API Context Detection
 * Following TDD principles - tests written FIRST
 * Constitution Compliance: Article III (TDD), Article XI.1 (Backend State Authority)
 */

// Import modules under test
const {
    checkTauriRuntime,
    initTauriWithFallback,
    safeTauriInvoke
} = require('../ui/btpc-tauri-context.js');

describe('Tauri API Context Detection', () => {
  let originalTauri;

  beforeEach(() => {
    // Save original Tauri state
    originalTauri = window.__TAURI__;
  });

  afterEach(() => {
    // Restore original Tauri state
    window.__TAURI__ = originalTauri;
  });

  describe('checkTauriRuntime', () => {
    test('should detect when Tauri runtime is available', () => {
      // Mock Tauri availability
      window.__TAURI__ = {
        invoke: jest.fn(),
        emit: jest.fn(),
        listen: jest.fn()
      };

      const result = checkTauriRuntime();

      expect(result).toBeDefined();
      expect(result.available).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('should detect when Tauri runtime is NOT available', () => {
      // Remove Tauri from window
      window.__TAURI__ = undefined;

      const result = checkTauriRuntime();

      expect(result).toBeDefined();
      expect(result.available).toBe(false);
      expect(result.error).toBeDefined();
    });

    test('should provide clear error message for browser context', () => {
      window.__TAURI__ = undefined;

      const result = checkTauriRuntime();

      expect(result.error).toContain('BTPC Wallet desktop app');
      expect(result.error).toContain('not browser');
    });

    test('should detect file:// protocol as invalid context', () => {
      window.__TAURI__ = undefined;

      // Mock file protocol
      Object.defineProperty(window, 'location', {
        value: { protocol: 'file:' },
        writable: true
      });

      const result = checkTauriRuntime();

      expect(result.available).toBe(false);
      expect(result.browserContext).toBe(true);
      expect(result.suggestion).toContain('Close this browser');
    });
  });

  describe('initTauriWithFallback', () => {
    test('should initialize Tauri when available', async () => {
      window.__TAURI__ = {
        invoke: jest.fn().mockResolvedValue({ initialized: true })
      };

      const result = await initTauriWithFallback();

      expect(result.success).toBe(true);
      expect(result.tauriAvailable).toBe(true);
    });

    test('should show user-friendly error when Tauri not available', async () => {
      window.__TAURI__ = undefined;

      const result = await initTauriWithFallback();

      expect(result.success).toBe(false);
      expect(result.tauriAvailable).toBe(false);
      expect(result.userAction).toBeDefined();
      expect(result.userAction).toContain('open BTPC Wallet');
    });

    test('should display warning banner when in browser context', async () => {
      window.__TAURI__ = undefined;
      document.body.innerHTML = '<div id="app"></div>';

      const result = await initTauriWithFallback();
      const banner = document.getElementById('tauri-warning-banner');

      expect(banner).toBeTruthy();
      expect(banner.textContent).toContain('desktop application');
    });

    test('should not show warning banner in Tauri context', async () => {
      window.__TAURI__ = {
        invoke: jest.fn().mockResolvedValue({ initialized: true })
      };
      document.body.innerHTML = '<div id="app"></div>';

      await initTauriWithFallback();
      const banner = document.getElementById('tauri-warning-banner');

      expect(banner).toBeFalsy();
    });
  });

  describe('safeTauriInvoke', () => {
    test('should invoke command when Tauri is available', async () => {
      const mockResult = { data: 'test' };
      window.__TAURI__ = {
        invoke: jest.fn().mockResolvedValue(mockResult)
      };

      const result = await safeTauriInvoke('test_command', { arg: 'value' });

      expect(result.success).toBe(true);
      expect(result.data).toEqual(mockResult);
      expect(window.__TAURI__.invoke).toHaveBeenCalledWith('test_command', { arg: 'value' });
    });

    test('should return error when Tauri is not available', async () => {
      window.__TAURI__ = undefined;

      const result = await safeTauriInvoke('test_command');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Tauri API not available');
      expect(result.requiresTauri).toBe(true);
    });

    test('should handle Tauri invoke errors gracefully', async () => {
      window.__TAURI__ = {
        invoke: jest.fn().mockRejectedValue(new Error('Command failed'))
      };

      const result = await safeTauriInvoke('test_command');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Command failed');
    });
  });

  describe('Constitution Compliance', () => {
    test('should comply with Article XI.1 - Backend State Authority', () => {
      // Verify that we're checking for backend availability
      window.__TAURI__ = undefined;

      const result = checkTauriRuntime();

      // When backend (Tauri) is not available, we should not allow operations
      expect(result.available).toBe(false);
      expect(result.error).toBeDefined();
    });

    test('should provide clear error messages per Article XI.4', () => {
      window.__TAURI__ = undefined;

      const result = checkTauriRuntime();

      // Error must explain what happened and what user should do
      expect(result.error.length).toBeGreaterThan(10);  // Clear explanation provided
      expect(result.suggestion || result.error).toContain('open');
    });
  });
});

// Custom Jest matchers
expect.extend({
  toBeValidTauriContext(received) {
    const pass = received &&
                  received.available === true &&
                  !received.error;

    return {
      pass,
      message: () => pass
        ? `Expected not to be valid Tauri context`
        : `Expected to be valid Tauri context but got: ${JSON.stringify(received)}`
    };
  }
});