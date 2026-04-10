/**
 * BTPC Professional Icons - Integration Example
 *
 * This file demonstrates how to integrate the professional icon set
 * into the BTPC wallet application.
 */

// ============================================================================
// SETUP
// ============================================================================

// Import the icon CSS in your main App.jsx or index.jsx
import './assets/icons-professional.css';

// ============================================================================
// ICON COMPONENT (Recommended)
// ============================================================================

/**
 * Reusable Icon component with type safety and consistent API
 */
const Icon = ({
  name,           // Icon name without 'icon-' prefix or '-pro' suffix
  size = 'base',  // 'sm' | 'base' | 'md' | 'lg'
  theme,          // 'primary' | 'success' | 'warning' | 'danger'
  interactive,    // Boolean - adds hover effects
  className,      // Additional CSS classes
  ...props
}) => {
  const classes = [
    `icon-${name}-pro`,
    `icon-${size}`,
    theme && `icon-${theme}`,
    interactive && 'icon-interactive',
    className
  ].filter(Boolean).join(' ');

  return <span className={classes} aria-hidden="true" {...props} />;
};

// ============================================================================
// TYPE DEFINITIONS (Optional - for TypeScript)
// ============================================================================

/**
 * Available icon names
 */
type IconName =
  | 'home'
  | 'wallet'
  | 'transactions'
  | 'mining'
  | 'node'
  | 'settings'
  | 'send'
  | 'receive'
  | 'address'
  | 'balance'
  | 'status'
  | 'security';

type IconSize = 'sm' | 'base' | 'md' | 'lg';
type IconTheme = 'primary' | 'success' | 'warning' | 'danger';

interface IconProps {
  name: IconName;
  size?: IconSize;
  theme?: IconTheme;
  interactive?: boolean;
  className?: string;
}

// ============================================================================
// USAGE EXAMPLES
// ============================================================================

// ----------------------------------------------------------------------------
// Example 1: Navigation Menu
// ----------------------------------------------------------------------------

const Navigation = () => {
  const navItems = [
    { icon: 'home', label: 'Dashboard', path: '/' },
    { icon: 'wallet', label: 'Wallet', path: '/wallet' },
    { icon: 'transactions', label: 'Transactions', path: '/transactions' },
    { icon: 'mining', label: 'Mining', path: '/mining' },
    { icon: 'node', label: 'Node', path: '/node' },
    { icon: 'settings', label: 'Settings', path: '/settings' }
  ];

  return (
    <nav className="sidebar-nav">
      {navItems.map(item => (
        <a
          key={item.path}
          href={item.path}
          className="nav-item"
        >
          <Icon name={item.icon} size="base" interactive />
          <span className="nav-label">{item.label}</span>
        </a>
      ))}
    </nav>
  );
};

// ----------------------------------------------------------------------------
// Example 2: Action Buttons
// ----------------------------------------------------------------------------

const WalletActions = () => {
  return (
    <div className="wallet-actions">
      <button className="btn btn-primary">
        <Icon name="send" size="base" />
        <span>Send BTPC</span>
      </button>

      <button className="btn btn-primary">
        <Icon name="receive" size="base" />
        <span>Receive BTPC</span>
      </button>

      <button className="btn btn-secondary">
        <Icon name="address" size="base" />
        <span>Show Address</span>
      </button>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 3: Dashboard Cards
// ----------------------------------------------------------------------------

const BalanceCard = ({ balance }) => {
  return (
    <div className="card balance-card">
      <div className="card-header">
        <Icon name="balance" size="md" theme="primary" />
        <h2>Total Balance</h2>
      </div>
      <div className="card-body">
        <p className="balance-amount">{balance} BTPC</p>
        <p className="balance-usd">${(balance * 10.5).toFixed(2)} USD</p>
      </div>
    </div>
  );
};

const MiningCard = ({ hashRate, status }) => {
  return (
    <div className="card mining-card">
      <div className="card-header">
        <Icon name="mining" size="md" theme="warning" />
        <h2>Mining Status</h2>
      </div>
      <div className="card-body">
        <div className="mining-status">
          <Icon
            name="status"
            size="sm"
            theme={status === 'active' ? 'success' : 'danger'}
          />
          <span>{status === 'active' ? 'Mining' : 'Inactive'}</span>
        </div>
        <p className="hash-rate">{hashRate} MH/s</p>
      </div>
    </div>
  );
};

const NodeCard = ({ peers, syncProgress }) => {
  return (
    <div className="card node-card">
      <div className="card-header">
        <Icon name="node" size="md" theme="success" />
        <h2>Network Status</h2>
      </div>
      <div className="card-body">
        <div className="node-info">
          <span>Connected Peers: {peers}</span>
          <span>Sync Progress: {syncProgress}%</span>
        </div>
      </div>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 4: Transaction List
// ----------------------------------------------------------------------------

const TransactionItem = ({ transaction }) => {
  const { type, amount, timestamp, status } = transaction;
  const isSend = type === 'send';

  return (
    <div className="transaction-item">
      <div className="transaction-icon">
        <Icon
          name={isSend ? 'send' : 'receive'}
          size="md"
          theme={isSend ? 'danger' : 'success'}
        />
      </div>

      <div className="transaction-details">
        <div className="transaction-type">
          {isSend ? 'Sent' : 'Received'}
        </div>
        <div className="transaction-time">
          {new Date(timestamp).toLocaleString()}
        </div>
      </div>

      <div className="transaction-amount">
        <span className={isSend ? 'negative' : 'positive'}>
          {isSend ? '-' : '+'}{amount} BTPC
        </span>
      </div>

      <div className="transaction-status">
        <Icon
          name="status"
          size="sm"
          theme={status === 'confirmed' ? 'success' : 'warning'}
        />
      </div>
    </div>
  );
};

const TransactionList = ({ transactions }) => {
  return (
    <div className="transaction-list">
      <div className="list-header">
        <Icon name="transactions" size="md" theme="primary" />
        <h2>Recent Transactions</h2>
      </div>
      <div className="list-body">
        {transactions.map((tx, index) => (
          <TransactionItem key={index} transaction={tx} />
        ))}
      </div>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 5: Settings Page
// ----------------------------------------------------------------------------

const SettingsSection = ({ icon, title, children }) => {
  return (
    <div className="settings-section">
      <div className="section-header">
        <Icon name={icon} size="md" theme="primary" />
        <h3>{title}</h3>
      </div>
      <div className="section-content">
        {children}
      </div>
    </div>
  );
};

const SettingsPage = () => {
  return (
    <div className="settings-page">
      <div className="page-header">
        <Icon name="settings" size="lg" />
        <h1>Settings</h1>
      </div>

      <SettingsSection icon="security" title="Security">
        <button className="settings-option">
          <Icon name="security" size="base" interactive />
          <span>Change Password</span>
        </button>
        <button className="settings-option">
          <Icon name="security" size="base" interactive />
          <span>Two-Factor Authentication</span>
        </button>
      </SettingsSection>

      <SettingsSection icon="wallet" title="Wallet">
        <button className="settings-option">
          <Icon name="address" size="base" interactive />
          <span>Backup Wallet</span>
        </button>
        <button className="settings-option">
          <Icon name="wallet" size="base" interactive />
          <span>Export Private Keys</span>
        </button>
      </SettingsSection>

      <SettingsSection icon="node" title="Network">
        <button className="settings-option">
          <Icon name="node" size="base" interactive />
          <span>Node Settings</span>
        </button>
        <button className="settings-option">
          <Icon name="status" size="base" interactive />
          <span>Connection Info</span>
        </button>
      </SettingsSection>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 6: Modal Dialogs
// ----------------------------------------------------------------------------

const SendModal = ({ isOpen, onClose, onSend }) => {
  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <Icon name="send" size="md" theme="primary" />
          <h2>Send BTPC</h2>
          <button onClick={onClose} className="modal-close">×</button>
        </div>

        <div className="modal-body">
          <div className="form-group">
            <label>
              <Icon name="address" size="sm" />
              Recipient Address
            </label>
            <input type="text" placeholder="Enter BTPC address" />
          </div>

          <div className="form-group">
            <label>
              <Icon name="balance" size="sm" />
              Amount
            </label>
            <input type="number" placeholder="0.00" />
          </div>
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={onSend}>
            <Icon name="send" size="base" />
            Send BTPC
          </button>
        </div>
      </div>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 7: Status Bar / Footer
// ----------------------------------------------------------------------------

const StatusBar = ({ connectionStatus, syncProgress, blockHeight }) => {
  return (
    <div className="status-bar">
      <div className="status-item">
        <Icon
          name="node"
          size="sm"
          theme={connectionStatus === 'connected' ? 'success' : 'danger'}
        />
        <span>{connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}</span>
      </div>

      <div className="status-item">
        <Icon name="status" size="sm" theme="primary" />
        <span>Block: {blockHeight.toLocaleString()}</span>
      </div>

      <div className="status-item">
        <Icon name="mining" size="sm" />
        <span>Sync: {syncProgress}%</span>
      </div>

      <div className="status-item">
        <Icon name="security" size="sm" theme="success" />
        <span>Encrypted</span>
      </div>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 8: Empty States
// ----------------------------------------------------------------------------

const EmptyTransactions = () => {
  return (
    <div className="empty-state">
      <Icon name="transactions" size="lg" theme="primary" />
      <h3>No Transactions Yet</h3>
      <p>Your transaction history will appear here</p>
      <div className="empty-actions">
        <button className="btn btn-primary">
          <Icon name="receive" size="base" />
          Receive BTPC
        </button>
      </div>
    </div>
  );
};

// ----------------------------------------------------------------------------
// Example 9: Notification / Toast
// ----------------------------------------------------------------------------

const Notification = ({ type, message, icon }) => {
  const themeMap = {
    success: 'success',
    error: 'danger',
    warning: 'warning',
    info: 'primary'
  };

  return (
    <div className={`notification notification-${type}`}>
      <Icon name={icon || 'status'} size="md" theme={themeMap[type]} />
      <span className="notification-message">{message}</span>
    </div>
  );
};

// Usage examples:
// <Notification type="success" icon="send" message="Transaction sent successfully!" />
// <Notification type="error" icon="security" message="Invalid password" />
// <Notification type="warning" icon="mining" message="Low hash rate detected" />

// ----------------------------------------------------------------------------
// Example 10: Complete Dashboard
// ----------------------------------------------------------------------------

const Dashboard = () => {
  return (
    <div className="dashboard">
      {/* Header */}
      <div className="dashboard-header">
        <Icon name="home" size="lg" theme="primary" />
        <h1>Dashboard</h1>
      </div>

      {/* Stats Cards */}
      <div className="dashboard-grid">
        <BalanceCard balance={1234.56} />
        <MiningCard hashRate={450.2} status="active" />
        <NodeCard peers={8} syncProgress={100} />

        <div className="card">
          <div className="card-header">
            <Icon name="security" size="md" theme="success" />
            <h2>Security Status</h2>
          </div>
          <div className="card-body">
            <p>Wallet Encrypted</p>
            <p>2FA Enabled</p>
          </div>
        </div>
      </div>

      {/* Transaction List */}
      <TransactionList transactions={[
        { type: 'receive', amount: 100, timestamp: Date.now(), status: 'confirmed' },
        { type: 'send', amount: 50, timestamp: Date.now() - 86400000, status: 'confirmed' }
      ]} />

      {/* Quick Actions */}
      <WalletActions />
    </div>
  );
};

// ============================================================================
// CSS EXAMPLE (Add to your stylesheet)
// ============================================================================

const exampleCSS = `
/* Icon Integration Styles */

/* Navigation */
.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  color: #e0e0e0;
  text-decoration: none;
  border-radius: 6px;
  transition: all 0.2s;
}

.nav-item:hover {
  background: rgba(102, 126, 234, 0.1);
  color: #667eea;
}

/* Buttons */
.btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 0.95rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(102, 126, 234, 0.3);
}

/* Cards */
.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

/* Transaction List */
.transaction-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  margin-bottom: 8px;
}

/* Status Bar */
.status-bar {
  display: flex;
  gap: 24px;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.2);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.85rem;
  color: #a0a0a0;
}
`;

// ============================================================================
// EXPORT
// ============================================================================

export {
  Icon,
  Navigation,
  WalletActions,
  BalanceCard,
  MiningCard,
  NodeCard,
  TransactionItem,
  TransactionList,
  SettingsPage,
  SendModal,
  StatusBar,
  EmptyTransactions,
  Notification,
  Dashboard
};

// Default export for convenience
export default Icon;