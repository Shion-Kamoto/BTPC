# BTPC Professional Icons - Quick Reference

## Installation

```bash
# Import in your CSS/JS
import 'assets/icons-professional.css';
```

## Basic Usage

```html
<span class="icon-[name]-pro icon-[size]"></span>
```

## Icon Names

| Icon | Class Name | Use Case |
|------|-----------|----------|
| 🏠 | `icon-home-pro` | Dashboard, Home |
| 👛 | `icon-wallet-pro` | Wallet, Portfolio |
| 🔄 | `icon-transactions-pro` | Transactions, History |
| ⛏️ | `icon-mining-pro` | Mining Operations |
| 🌐 | `icon-node-pro` | Network, Node Status |
| ⚙️ | `icon-settings-pro` | Settings, Config |
| 📤 | `icon-send-pro` | Send, Transfer |
| 📥 | `icon-receive-pro` | Receive Funds |
| 🏷️ | `icon-address-pro` | Address, QR Code |
| 💰 | `icon-balance-pro` | Balance, Funds |
| ℹ️ | `icon-status-pro` | Status, Info |
| 🔒 | `icon-security-pro` | Security, Lock |

## Size Classes

```html
<span class="icon-wallet-pro icon-sm"></span>    <!-- 16px -->
<span class="icon-wallet-pro icon-base"></span>  <!-- 20px (default) -->
<span class="icon-wallet-pro icon-md"></span>    <!-- 24px -->
<span class="icon-wallet-pro icon-lg"></span>    <!-- 32px -->
```

## Theme Classes

```html
<span class="icon-wallet-pro icon-base icon-primary"></span>  <!-- Blue accent -->
<span class="icon-send-pro icon-base icon-success"></span>    <!-- Green -->
<span class="icon-status-pro icon-base icon-warning"></span>  <!-- Yellow -->
<span class="icon-receive-pro icon-base icon-danger"></span>  <!-- Red -->
```

## Interactive Icons

```html
<button>
  <span class="icon-settings-pro icon-base icon-interactive"></span>
  Settings
</button>
```

## React Example

```jsx
// Simple Icon Component
const Icon = ({ name, size = 'base', theme, interactive }) => {
  const classes = [
    `icon-${name}-pro`,
    `icon-${size}`,
    theme && `icon-${theme}`,
    interactive && 'icon-interactive'
  ].filter(Boolean).join(' ');

  return <span className={classes} aria-hidden="true" />;
};

// Usage
<Icon name="wallet" size="md" theme="primary" />
<Icon name="send" interactive />
```

## Common Patterns

### Navigation Item
```jsx
<a href="/wallet" className="nav-item">
  <Icon name="wallet" size="base" interactive />
  <span>Wallet</span>
</a>
```

### Action Button
```jsx
<button className="btn-primary">
  <Icon name="send" size="base" />
  Send BTPC
</button>
```

### Status Indicator
```jsx
<div className="status">
  <Icon name="status" size="sm" theme="success" />
  <span>Connected</span>
</div>
```

### Card Header
```jsx
<div className="card-header">
  <Icon name="balance" size="md" theme="primary" />
  <h2>Total Balance</h2>
</div>
```

## Best Practices

✅ **DO:**
- Use icon-base (20px) for standard UI elements
- Add aria-label to buttons with only icons
- Use icon-interactive for clickable icons
- Test visibility at minimum size (16px)

❌ **DON'T:**
- Use icons smaller than 14px
- Forget to set aria-hidden="true" on decorative icons
- Mix old and new icon sets
- Override stroke-width (breaks consistency)

## Accessibility

```html
<!-- Decorative icon (no alt needed) -->
<button aria-label="Settings">
  <span class="icon-settings-pro icon-base" aria-hidden="true"></span>
</button>

<!-- Icon with visible text -->
<button>
  <span class="icon-send-pro icon-base" aria-hidden="true"></span>
  Send
</button>
```

## Custom Colors

```css
/* Custom theme in your CSS */
.icon-custom {
  color: #ff6b6b;
}

.icon-custom:hover {
  color: #ff5252;
}
```

```html
<span class="icon-wallet-pro icon-base icon-custom"></span>
```

## File Locations

- **CSS**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/icons-professional.css`
- **SVG Sources**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/icons-svg/`
- **Demo**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/icons-demo.html`
- **Guide**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/ICON_DESIGN_GUIDE.md`

## Quick Testing

Open the demo file in your browser:
```bash
open /home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/icons-demo.html
```

## Performance

- Total CSS size: ~15KB uncompressed, ~5KB gzipped
- Zero additional HTTP requests
- Data URIs cached with CSS
- Optimal for production builds