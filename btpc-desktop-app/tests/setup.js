/**
 * Jest Test Setup
 *
 * This file configures the testing environment for the BTPC Desktop App frontend.
 * It sets up DOM testing utilities and mocks for Tauri APIs.
 */

require('@testing-library/jest-dom');

// Mock Tauri API since it's not available in test environment
global.__TAURI__ = {
  invoke: jest.fn(),
  event: {
    listen: jest.fn(),
    emit: jest.fn(),
  },
  window: {
    appWindow: {
      minimize: jest.fn(),
      maximize: jest.fn(),
      close: jest.fn(),
    }
  }
};

// Mock Tauri invoke function with common app commands
const mockInvoke = jest.fn((command, args) => {
  console.log(`[TEST] Mocked Tauri invoke: ${command}`, args);

  switch (command) {
    case 'get_system_status':
      return Promise.resolve({
        node_status: 'Stopped',
        node_pid: null,
        wallet_balance: '0 base units (0.00000000 BTP)',
        mining_status: 'Stopped',
        binaries_installed: true,
        config_exists: true,
        logs_available: []
      });

    case 'test_command':
      return Promise.resolve('Test command works!');

    case 'get_wallet_balance':
      return Promise.resolve('3237500000 base units (32.37500000 BTP)');

    case 'get_wallet_address':
      return Promise.resolve('a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890');

    case 'create_wallet':
      return Promise.resolve('Wallet created successfully');

    case 'start_node':
      return Promise.resolve('Node started successfully (PID: 12345)');

    case 'stop_node':
      return Promise.resolve('Node stopped successfully');

    case 'start_mining':
      return Promise.resolve('Mining started: 5 blocks to address');

    case 'stop_mining':
      return Promise.resolve('Mining stopped successfully');

    case 'send_btpc':
      return Promise.resolve('Transaction created successfully');

    case 'setup_btpc':
      return Promise.resolve('BTPC setup completed');

    case 'login_user':
      return Promise.resolve({
        session_id: 'test-session-123',
        username: 'testuser',
        expires_at: Date.now() + 30 * 60 * 1000 // 30 minutes
      });

    case 'logout_user':
      return Promise.resolve('Successfully logged out');

    case 'check_session':
      return Promise.resolve(true);

    case 'get_utxo_stats':
      return Promise.resolve({
        total_utxos: 5,
        total_value_credits: 16187500000,
        total_value_btp: 161.875,
        spendable_utxos: 3,
        spendable_value_credits: 9712500000,
        spendable_value_btp: 97.125
      });

    default:
      console.warn(`[TEST] Unhandled Tauri command: ${command}`);
      return Promise.reject(new Error(`Unhandled command: ${command}`));
  }
});

global.__TAURI__.invoke = mockInvoke;

// Setup DOM for testing
Object.defineProperty(window, 'location', {
  value: {
    href: 'http://localhost:3000',
    origin: 'http://localhost:3000',
    reload: jest.fn()
  },
  writable: true
});

// Mock localStorage
const localStorageMock = {
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  clear: jest.fn(),
  length: 0,
  key: jest.fn()
};
global.localStorage = localStorageMock;

// Mock sessionStorage
const sessionStorageMock = {
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  clear: jest.fn(),
  length: 0,
  key: jest.fn()
};
global.sessionStorage = sessionStorageMock;

// Mock console methods for cleaner test output
global.console = {
  ...console,
  warn: jest.fn(),
  error: jest.fn(),
  info: jest.fn(),
  debug: jest.fn()
};

// Mock window.alert and confirm
global.alert = jest.fn();
global.confirm = jest.fn(() => true);

// Mock file operations
global.File = class MockFile {
  constructor(bits, filename, options = {}) {
    this.bits = bits;
    this.name = filename;
    this.size = bits.reduce((acc, bit) => acc + bit.length, 0);
    this.type = options.type || '';
    this.lastModified = Date.now();
  }
};

// Setup fetch mock for network requests
global.fetch = jest.fn(() =>
  Promise.resolve({
    json: () => Promise.resolve({}),
    text: () => Promise.resolve(''),
    ok: true,
    status: 200
  })
);

// Mock IntersectionObserver
global.IntersectionObserver = class IntersectionObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  unobserve() {}
};

// Mock ResizeObserver
global.ResizeObserver = class ResizeObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  unobserve() {}
};

// Mock crypto for secure random generation
Object.defineProperty(global, 'crypto', {
  value: {
    getRandomValues: (arr) => {
      for (let i = 0; i < arr.length; i++) {
        arr[i] = Math.floor(Math.random() * 256);
      }
      return arr;
    }
  }
});

// Add custom matchers for testing BTPC-specific functionality
expect.extend({
  toBeBtpcAddress(received) {
    const pass = typeof received === 'string' &&
                 received.length === 128 &&
                 /^[a-fA-F0-9]+$/.test(received);

    if (pass) {
      return {
        message: () => `expected ${received} not to be a valid BTPC address`,
        pass: true,
      };
    } else {
      return {
        message: () => `expected ${received} to be a valid BTPC address (128 hex characters)`,
        pass: false,
      };
    }
  },

  toBeBtpAmount(received) {
    const pass = typeof received === 'string' &&
                 /^\d+ base units \(\d+\.\d{8} BTP\)$/.test(received);

    if (pass) {
      return {
        message: () => `expected ${received} not to be a valid BTP amount format`,
        pass: true,
      };
    } else {
      return {
        message: () => `expected ${received} to be a valid BTP amount format`,
        pass: false,
      };
    }
  }
});

// Silence specific warnings for cleaner test output
const originalError = console.error;
beforeAll(() => {
  console.error = (...args) => {
    if (
      typeof args[0] === 'string' &&
      args[0].includes('Warning: ReactDOM.render is deprecated')
    ) {
      return;
    }
    originalError.call(console, ...args);
  };
});

afterAll(() => {
  console.error = originalError;
});