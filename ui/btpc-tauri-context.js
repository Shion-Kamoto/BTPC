/**
 * BTPC Tauri Context Detection and Management
 *
 * This module ensures the application runs in the correct Tauri context
 * and provides fallbacks for browser environments.
 *
 * Constitution Compliance:
 * - Article XI.1: Backend State Authority
 * - Article XI.4: Clear Error Messages
 */

/**
 * Check if Tauri runtime is available
 * @returns {Object} Status of Tauri availability
 */
function checkTauriRuntime() {
  // Check for Tauri runtime: __TAURI_INTERNALS__ (most reliable, always first) → __TAURI__ → __TAURI_INVOKE__
  const isTauriAvailable = typeof window !== 'undefined' &&
                           (typeof window.__TAURI_INTERNALS__ !== 'undefined' ||
                            typeof window.__TAURI_INVOKE__ === 'function' ||
                            (typeof window.__TAURI__ !== 'undefined' && window.__TAURI__ !== null));

  if (isTauriAvailable) {
    return {
      available: true,
      tauriVersion: window.__TAURI__?.version || '2.0'
    };
  }

  // Check if we're in a browser context
  const isBrowserContext = typeof window !== 'undefined' &&
                           window.location &&
                           window.location.protocol === 'file:';

  const error = 'Application must be opened through BTPC Wallet desktop app, not browser';
  const suggestion = isBrowserContext
    ? 'Close this browser window and open the BTPC Wallet desktop application'
    : 'Please launch the BTPC Wallet application from your desktop';

  return {
    available: false,
    error: error,
    suggestion: suggestion,
    browserContext: isBrowserContext
  };
}

/**
 * Initialize Tauri with fallback for browser context
 * @returns {Promise<Object>} Initialization result
 */
async function initTauriWithFallback() {
  const tauriCheck = checkTauriRuntime();

  if (tauriCheck.available) {
    try {
      // Verify Tauri is truly functional - use window.invoke which was set up earlier
      if (window.invoke) {
        await window.invoke('ping').catch(() => {
          // If ping fails, the command might not exist, but Tauri is available
        });
      }

      return {
        success: true,
        tauriAvailable: true
      };
    } catch (error) {
      console.error('Tauri initialization error:', error);
      return {
        success: false,
        tauriAvailable: true,
        error: error.message
      };
    }
  }

  // Tauri not available - show warning
  displayTauriWarning();

  return {
    success: false,
    tauriAvailable: false,
    userAction: 'Please close this window and open BTPC Wallet desktop application',
    error: tauriCheck.error
  };
}

/**
 * Display a warning banner when running in browser context
 */
function displayTauriWarning() {
  // Don't add multiple warnings
  if (document.getElementById('tauri-warning-banner')) {
    return;
  }

  const warningBanner = document.createElement('div');
  warningBanner.id = 'tauri-warning-banner';
  warningBanner.className = 'tauri-warning-banner';
  warningBanner.innerHTML = `
    <div class="warning-content">
      <span class="warning-icon">⚠️</span>
      <div class="warning-text">
        <strong>Wrong Context Detected</strong>
        <p>You're viewing this in a browser. Please use the BTPC Wallet desktop application instead.</p>
        <p class="warning-action">Close this browser window and launch BTPC Wallet from your desktop.</p>
      </div>
    </div>
  `;

  // Add styles
  const style = document.createElement('style');
  style.textContent = `
    .tauri-warning-banner {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      background: linear-gradient(135deg, #ff6b6b 0%, #ff5252 100%);
      color: white;
      padding: 15px;
      z-index: 10000;
      box-shadow: 0 2px 10px rgba(0,0,0,0.3);
      animation: slideDown 0.3s ease-out;
    }

    .warning-content {
      display: flex;
      align-items: center;
      max-width: 1200px;
      margin: 0 auto;
      gap: 15px;
    }

    .warning-icon {
      font-size: 2em;
      animation: pulse 2s infinite;
    }

    .warning-text {
      flex: 1;
    }

    .warning-text strong {
      display: block;
      font-size: 1.2em;
      margin-bottom: 5px;
    }

    .warning-text p {
      margin: 3px 0;
      font-size: 0.95em;
    }

    .warning-action {
      font-weight: 600;
      text-decoration: underline;
    }

    @keyframes slideDown {
      from {
        transform: translateY(-100%);
        opacity: 0;
      }
      to {
        transform: translateY(0);
        opacity: 1;
      }
    }

    @keyframes pulse {
      0%, 100% { transform: scale(1); }
      50% { transform: scale(1.1); }
    }

    /* Adjust body to account for banner */
    body.has-tauri-warning {
      padding-top: 80px;
    }
  `;

  document.head.appendChild(style);
  document.body.insertBefore(warningBanner, document.body.firstChild);
  document.body.classList.add('has-tauri-warning');
}

/**
 * Safe wrapper for Tauri invoke calls
 * @param {string} command - The Tauri command to invoke
 * @param {Object} args - Arguments for the command
 * @returns {Promise<Object>} Result with success status
 */
async function safeTauriInvoke(command, args = {}) {
  const tauriCheck = checkTauriRuntime();

  if (!tauriCheck.available) {
    console.warn(`Cannot invoke '${command}': Tauri API not available`);
    return {
      success: false,
      error: 'Tauri API not available. Please use the desktop application.',
      requiresTauri: true
    };
  }

  try {
    // Call the actual Tauri API directly to avoid infinite recursion
    // Priority: __TAURI_INTERNALS__ (most reliable) → __TAURI__.core → fallbacks
    let result;
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
      result = await window.__TAURI_INTERNALS__.invoke(command, args);
    } else if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      result = await window.__TAURI__.core.invoke(command, args);
    } else if (typeof window.__TAURI_INVOKE__ === 'function') {
      result = await window.__TAURI_INVOKE__(command, args);
    } else if (window.__TAURI__ && typeof window.__TAURI__.invoke === 'function') {
      result = await window.__TAURI__.invoke(command, args);
    } else {
      throw new Error('Tauri invoke API not found');
    }

    return {
      success: true,
      data: result
    };
  } catch (error) {
    console.error(`Tauri invoke error for '${command}':`, error);
    return {
      success: false,
      error: error.message || 'Command failed',
      command: command
    };
  }
}

/**
 * Safe wrapper for Tauri event listening
 * @param {string} event - Event name to listen for
 * @param {Function} handler - Event handler function
 * @returns {Function|null} Unlisten function or null if Tauri not available
 */
function safeTauriListen(event, handler) {
  const tauriCheck = checkTauriRuntime();

  if (!tauriCheck.available) {
    console.warn(`Cannot listen to '${event}': Tauri API not available`);
    return null;
  }

  try {
    return window.__TAURI__.listen(event, handler);
  } catch (error) {
    console.error(`Tauri listen error for '${event}':`, error);
    return null;
  }
}

/**
 * Safe wrapper for Tauri event emission
 * @param {string} event - Event name to emit
 * @param {*} payload - Event payload
 * @returns {boolean} Success status
 */
function safeTauriEmit(event, payload) {
  const tauriCheck = checkTauriRuntime();

  if (!tauriCheck.available) {
    console.warn(`Cannot emit '${event}': Tauri API not available`);
    return false;
  }

  try {
    window.__TAURI__.emit(event, payload);
    return true;
  } catch (error) {
    console.error(`Tauri emit error for '${event}':`, error);
    return false;
  }
}

// Expose window.invoke IMMEDIATELY (before DOMContentLoaded) for all Tauri versions.
// Uses a LAZY proxy pattern — resolves the Tauri API at call time, not load time.
// Workaround for Tauri bug #12990: window.__TAURI__ is undefined in top-level scripts
// but window.__TAURI_INTERNALS__ IS available (injected at WebView init level).
if (typeof window !== 'undefined') {
  window.invoke = function(cmd, args) {
    // Resolve the correct invoke function at CALL TIME (lazy)
    // Priority: __TAURI_INTERNALS__ (most reliable, available earliest) → __TAURI__.core → fallbacks
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
      return window.__TAURI_INTERNALS__.invoke(cmd, args);
    }
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      return window.__TAURI__.core.invoke(cmd, args);
    }
    if (typeof window.__TAURI_INVOKE__ === 'function') {
      return window.__TAURI_INVOKE__(cmd, args);
    }
    if (window.__TAURI__ && typeof window.__TAURI__.invoke === 'function') {
      return window.__TAURI__.invoke(cmd, args);
    }
    return Promise.reject(new Error('[Tauri Init] No Tauri API found — invoke unavailable for: ' + cmd));
  };
  console.log('[Tauri Init] Lazy invoke proxy installed (resolves API at call time)');

  // Lazy proxy for event.listen — resolves at call time with retry if __TAURI__ not ready yet
  window.tauriListen = function(event, handler) {
    if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === 'function') {
      return window.__TAURI__.event.listen(event, handler);
    }
    // __TAURI__ not ready yet — wait up to 5 seconds with polling
    console.log('[Tauri Init] Event API not ready, waiting for: ' + event);
    return new Promise(function(resolve) {
      var attempts = 0;
      var maxAttempts = 50; // 50 × 100ms = 5s
      var timer = setInterval(function() {
        attempts++;
        if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === 'function') {
          clearInterval(timer);
          window.__TAURI__.event.listen(event, handler).then(resolve);
        } else if (attempts >= maxAttempts) {
          clearInterval(timer);
          console.warn('[Tauri Init] Event API unavailable after 5s for: ' + event);
          resolve(function() {}); // Return no-op unlisten
        }
      }, 100);
    });
  };

  // Lazy proxy for event.emit — with retry
  window.tauriEmit = function(event, payload) {
    if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.emit === 'function') {
      return window.__TAURI__.event.emit(event, payload);
    }
    if (window.__TAURI__ && typeof window.__TAURI__.emit === 'function') {
      return window.__TAURI__.emit(event, payload);
    }
    // Retry after short delay
    setTimeout(function() {
      if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.emit === 'function') {
        window.__TAURI__.event.emit(event, payload);
      } else {
        console.warn('[Tauri Init] Emit API not available for: ' + event);
      }
    }, 500);
  };
}

/**
 * Initialize Tauri context on page load
 */
document.addEventListener('DOMContentLoaded', async () => {
  const initResult = await initTauriWithFallback();

  if (!initResult.success) {
    console.error('⚠️ BTPC Wallet: Running in incorrect context');
    console.info('✅ Solution:', initResult.userAction);

    // Disable all Tauri-dependent features
    document.querySelectorAll('button[data-requires-tauri="true"]').forEach(button => {
      button.disabled = true;
      button.title = 'Requires BTPC Wallet desktop application';
    });
  } else {
    console.info('✅ BTPC Wallet: Tauri context initialized successfully');
  }

  // Make functions globally available
  window.btpcTauri = {
    checkRuntime: checkTauriRuntime,
    init: initTauriWithFallback,
    invoke: safeTauriInvoke,
    listen: safeTauriListen,
    emit: safeTauriEmit
  };
});

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    checkTauriRuntime,
    initTauriWithFallback,
    safeTauriInvoke,
    safeTauriListen,
    safeTauriEmit
  };
}