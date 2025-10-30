# BTPC Professional Icon Set - Design Guide

## Overview
Professional, bold icon set designed specifically for cryptocurrency wallet applications. Optimized for high visibility at 16-20px sizes with strategic use of fills and bold strokes.

## Design Specifications

### Technical Parameters
- **ViewBox**: 24x24
- **Stroke Weight**: 2.5px (with 3px for critical elements)
- **Stroke Caps**: Round
- **Stroke Joins**: Round
- **Color**: `stroke='currentColor'` for dynamic theming
- **Fill Strategy**: Strategic fills with 15-80% opacity for depth and contrast

### Visual Principles
1. **High Contrast**: Filled shapes combined with strokes
2. **Bold Lines**: Minimum 2.5px, up to 3px for emphasis
3. **Simple Geometry**: No fine details that vanish at small sizes
4. **Professional Aesthetic**: Financial application appropriate
5. **Instant Recognition**: Each icon unique and distinctive
6. **Consistent Style**: Cohesive design language across set

## Icon Catalog

### 1. icon-home-pro
**Purpose**: Dashboard/Home navigation
**Design**: Bold house with filled interior and prominent roof
**Key Features**:
- Semi-transparent fill (15% opacity) for interior
- Thick stroke outlines
- Clear door indicator
**Use Cases**: Main dashboard, home navigation

### 2. icon-wallet-pro
**Purpose**: Wallet/Portfolio access
**Design**: Wallet with filled body and solid circular lock/clasp
**Key Features**:
- Filled wallet body (15% opacity)
- Solid filled circle (100% opacity) for lock
- Flap detail at top
**Use Cases**: Wallet overview, portfolio page

### 3. icon-transactions-pro
**Purpose**: Transaction history/activity
**Design**: Dual arrows (up/down) with filled arrow heads
**Key Features**:
- Filled arrow heads (80% opacity) for maximum visibility
- Bold directional lines
- Clear vertical separation
**Use Cases**: Transaction list, history view, activity feed

### 4. icon-mining-pro
**Purpose**: Mining operations
**Design**: Pickaxe crossed with blockchain cube
**Key Features**:
- Filled pickaxe head (20% opacity)
- Filled cube with grid (30% opacity)
- Strong diagonal composition
**Use Cases**: Mining dashboard, pool status, hash rate display

### 5. icon-node-pro
**Purpose**: Network/Node status
**Design**: Network topology with central hub and satellites
**Key Features**:
- Filled center node (100% opacity)
- Filled satellite nodes (80% opacity)
- Bold connecting lines
**Use Cases**: Node management, network status, peer connections

### 6. icon-settings-pro
**Purpose**: Settings/Configuration
**Design**: Gear/cog with radiating spokes
**Key Features**:
- Filled center circle (100% opacity)
- Bold radiating spokes (3px stroke)
- 8-point design for clarity
**Use Cases**: Settings menu, configuration, preferences

### 7. icon-send-pro
**Purpose**: Send/Transfer funds
**Design**: Upward arrow emerging from envelope/box
**Key Features**:
- Filled envelope (10% opacity)
- Solid filled arrow head (100% opacity)
- Clear upward-right diagonal
**Use Cases**: Send transaction, transfer funds, payment

### 8. icon-receive-pro
**Purpose**: Receive funds
**Design**: Downward arrow entering envelope/box
**Key Features**:
- Filled envelope (10% opacity)
- Solid filled arrow head (100% opacity)
- Clear downward-left diagonal
**Use Cases**: Receive transaction, generate address, request payment

### 9. icon-address-pro
**Purpose**: Address/QR code display
**Design**: Simplified QR code pattern in frame
**Key Features**:
- Filled frame (10% opacity)
- Solid filled squares in recognizable QR pattern
- 8 distinct filled squares
**Use Cases**: Address display, QR code generation, wallet address

### 10. icon-balance-pro
**Purpose**: Balance/Funds display
**Design**: Stacked coins with 3D perspective
**Key Features**:
- Three ellipses with graduated opacity (20%-40%)
- Currency symbol integrated
- 3D depth effect
**Use Cases**: Balance display, portfolio value, account summary

### 11. icon-status-pro
**Purpose**: Status/Information
**Design**: Circle with information symbol
**Key Features**:
- Filled circle background (15% opacity)
- Bold i-dot (3px stroke for line, filled circle for dot)
- High contrast
**Use Cases**: Status indicators, info tooltips, notifications

### 12. icon-security-pro
**Purpose**: Security/Lock features
**Design**: Padlock with thick body and shackle
**Key Features**:
- Filled lock body (20% opacity)
- Bold shackle (3px stroke)
- Keyhole detail with filled circle
**Use Cases**: Security settings, encrypted wallet, password protection

## Usage Examples

### Basic Implementation
```html
<!-- Simple icon -->
<span class="icon-home-pro icon-base"></span>

<!-- With size variant -->
<span class="icon-wallet-pro icon-md"></span>

<!-- With theme -->
<span class="icon-send-pro icon-base icon-primary"></span>

<!-- Interactive icon -->
<button>
  <span class="icon-settings-pro icon-base icon-interactive"></span>
  Settings
</button>
```

### React Component
```jsx
const Icon = ({ name, size = 'base', theme, interactive }) => {
  const classes = [
    `icon-${name}-pro`,
    `icon-${size}`,
    theme && `icon-${theme}`,
    interactive && 'icon-interactive'
  ].filter(Boolean).join(' ');

  return <span className={classes} />;
};

// Usage
<Icon name="wallet" size="md" theme="primary" />
```

### Navigation Menu
```jsx
const navItems = [
  { icon: 'home', label: 'Dashboard', path: '/' },
  { icon: 'wallet', label: 'Wallet', path: '/wallet' },
  { icon: 'transactions', label: 'Transactions', path: '/transactions' },
  { icon: 'mining', label: 'Mining', path: '/mining' },
  { icon: 'node', label: 'Node', path: '/node' },
  { icon: 'settings', label: 'Settings', path: '/settings' }
];

return (
  <nav>
    {navItems.map(item => (
      <a key={item.path} href={item.path}>
        <span className={`icon-${item.icon}-pro icon-base icon-interactive`} />
        {item.label}
      </a>
    ))}
  </nav>
);
```

### Transaction Actions
```jsx
<div className="transaction-actions">
  <button className="btn-send">
    <span className="icon-send-pro icon-md" />
    Send
  </button>
  <button className="btn-receive">
    <span className="icon-receive-pro icon-md" />
    Receive
  </button>
</div>
```

## Size Guidelines

### Recommended Sizes
- **icon-sm** (16px): Inline text, dense lists, footer
- **icon-base** (20px): Navigation, buttons, standard UI
- **icon-md** (24px): Featured buttons, page headers
- **icon-lg** (32px): Hero sections, empty states

### Minimum Sizes
- Never use below 14px - details become unclear
- For 14-15px use icon-sm with extra bold theme
- Optimal range: 18-28px

## Theme Variants

### Standard Themes
- **default**: Natural currentColor
- **primary**: Enhanced brightness/saturation (+20%/+30%)
- **success**: Green hue shift (80deg)
- **warning**: Yellow hue shift (30deg)
- **danger**: Red saturation boost (+40%)

### Custom Theming
```css
/* Add to your theme CSS */
.dark-theme .icon-base {
  color: #e0e0e0;
}

.dark-theme .icon-primary {
  color: #4a9eff;
}

.light-theme .icon-base {
  color: #2c3e50;
}
```

## Accessibility

### Color Contrast
- All icons meet WCAG AA standards at 4.5:1 contrast
- Strategic fills ensure visibility on dark backgrounds
- Icons remain legible with color blindness filters

### Alternative Text
Always provide text labels or aria-labels:
```html
<button aria-label="Settings">
  <span className="icon-settings-pro icon-base" aria-hidden="true" />
</button>
```

## Performance

### Optimization
- SVG data URIs are gzipped automatically by browsers
- Average icon size: ~600-800 bytes encoded
- Total CSS file: ~15KB uncompressed, ~5KB gzipped
- Zero network requests after CSS load

### Loading Strategy
```jsx
// Preload in app initialization
import 'assets/icons-professional.css';

// Or lazy load for code splitting
const loadIcons = () => import('assets/icons-professional.css');
```

## Migration from Old Icons

### Find and Replace
```bash
# Update class names
sed -i 's/icon-home/icon-home-pro/g' src/**/*.jsx
sed -i 's/icon-wallet/icon-wallet-pro/g' src/**/*.jsx
# ... repeat for all icons
```

### Gradual Migration
```css
/* Keep old icons as aliases during transition */
.icon-home { @extend .icon-home-pro; }
.icon-wallet { @extend .icon-wallet-pro; }
```

## Design Files

### Source SVG Location
Individual SVG files available at:
`/home/bob/BTPC/BTPC/btpc-desktop-app/ui/src/assets/icons-svg/`

### Editing Guidelines
1. Maintain 24x24 viewBox
2. Keep stroke-width at 2.5px minimum
3. Use fill-opacity for depth (15%-80%)
4. Test at 16px, 20px, 24px sizes
5. Verify in light and dark themes

## Version History
- **v1.0** (2025-10-12): Initial professional icon set
  - 12 core icons
  - Bold design language
  - Strategic fills for visibility
  - Cryptocurrency wallet optimized

## Credits
Designed for BTPC Cryptocurrency Wallet
Optimized for Tauri desktop application
Based on industry standards from Exodus, Monero GUI, Trust Wallet