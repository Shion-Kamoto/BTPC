# BTPC Desktop Application Style Guide

**Last Updated:** 2025-10-07
**Version:** 2.0 (Post-Redesign)
**Status:** Production Ready

## Overview

This style guide defines the visual design system for the BTPC (Bitcoin Post-Quantum Cryptocurrency) desktop application. It establishes a cohesive design language with a modern quantum-themed aesthetic, drawing inspiration from Monero GUI wallet's battle-tested user experience patterns while adapting to BTPC's unique quantum-resistant branding.

### Key Design Principles
- **Clarity**: Every UI element should have a clear purpose and label
- **Security-First**: Sensitive information (seeds, keys) must have clear warnings
- **Progressive Disclosure**: Essential features first, advanced options accessible
- **Feedback**: Immediate visual feedback for all user actions
- **Accessibility**: Full ARIA labels and keyboard navigation support
- **Quantum Theme**: Indigo/purple color palette reflecting quantum computing
- **Professional**: Clean, modern aesthetic suitable for production deployment

## Brand Identity

### BTPC Logo Usage
- **Primary Logo**: Use BTPC full wordmark with blockchain symbol
- **Icon Version**: Circular BTPC symbol for favicons and small applications
- **Minimum Size**: 32px for digital applications
- **Clear Space**: Minimum 1x logo height on all sides
- **Dark Background**: Use white or BTPC Gold logo variants
- **Light Background**: Use BTPC Blue logo variant

### Brand Colors

#### Primary Palette
```css
/* Primary Brand Colors */
--btpc-blue: #1a365d;        /* Primary brand color */
--btpc-gold: #ffd700;        /* Accent and highlight color */
--btpc-blue-light: #2c5282; /* Lighter variant for hover states */
--btpc-blue-dark: #153e75;   /* Darker variant for pressed states */
```

#### Terminal Palette
```css
/* Terminal-inspired Status Colors */
--terminal-green: #48bb78;   /* Success, active, connected states */
--terminal-amber: #ed8936;   /* Warning, processing states */
--terminal-red: #f56565;     /* Error, disconnected, stop states */
--terminal-cyan: #38b2ac;    /* Information, links */
--terminal-purple: #9f7aea;  /* Special actions, premium features */
```

#### Neutral Palette
```css
/* Dark Theme Base */
--bg-primary: #1a202c;       /* Main application background */
--bg-secondary: #2d3748;     /* Sidebar, cards, elevated surfaces */
--bg-tertiary: #374151;      /* Hover states, subtle emphasis */
--border-color: #4a5568;     /* Borders, dividers */
--text-primary: #e2e8f0;     /* Primary text, high emphasis */
--text-secondary: #a0aec0;   /* Secondary text, medium emphasis */
--text-muted: #718096;       /* Disabled text, low emphasis */
```

## Typography

### Font Hierarchy

#### Primary Typeface: Fira Code
- **Usage**: Primary interface text, code displays, addresses
- **Weights**: Light (300), Regular (400), Medium (500), Bold (700)
- **Character Set**: Full Latin, symbols, ligatures for code

#### Fallback Stack
```css
font-family: 'Fira Code', 'SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', 'Consolas', 'Courier New', monospace;
```

### Text Styles

#### Headings
```css
/* H1 - Page Titles */
.text-h1 {
  font-size: 2rem;      /* 32px */
  font-weight: 700;
  line-height: 1.2;
  letter-spacing: -0.025em;
  color: var(--text-primary);
}

/* H2 - Section Headers */
.text-h2 {
  font-size: 1.5rem;    /* 24px */
  font-weight: 600;
  line-height: 1.3;
  color: var(--text-primary);
}

/* H3 - Subsection Headers */
.text-h3 {
  font-size: 1.25rem;   /* 20px */
  font-weight: 500;
  line-height: 1.4;
  color: var(--text-primary);
}
```

#### Body Text
```css
/* Body Large */
.text-body-lg {
  font-size: 1rem;      /* 16px */
  font-weight: 400;
  line-height: 1.5;
  color: var(--text-primary);
}

/* Body Regular */
.text-body {
  font-size: 0.875rem;  /* 14px */
  font-weight: 400;
  line-height: 1.5;
  color: var(--text-primary);
}

/* Body Small */
.text-body-sm {
  font-size: 0.75rem;   /* 12px */
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-secondary);
}
```

#### Specialized Text
```css
/* Code/Addresses */
.text-code {
  font-size: 0.8rem;    /* 13px */
  font-weight: 400;
  font-family: 'Fira Code', monospace;
  color: var(--terminal-cyan);
  background: rgba(56, 178, 172, 0.1);
  padding: 2px 6px;
  border-radius: 4px;
}

/* Labels */
.text-label {
  font-size: 0.75rem;   /* 12px */
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}
```

## Component Library

### Buttons

#### Primary Buttons
```css
.btn-primary {
  background: var(--btpc-blue);
  color: white;
  border: none;
  padding: 12px 24px;
  border-radius: 8px;
  font-weight: 500;
  font-size: 0.875rem;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.btn-primary:hover {
  background: var(--btpc-blue-light);
  transform: translateY(-1px);
}

.btn-primary:active {
  background: var(--btpc-blue-dark);
  transform: translateY(0);
}
```

#### Action-Specific Buttons
```css
/* Success Actions (Start, Connect, etc.) */
.btn-success {
  background: var(--terminal-green);
  color: white;
}

/* Warning Actions (Restart, Reset, etc.) */
.btn-warning {
  background: var(--terminal-amber);
  color: white;
}

/* Danger Actions (Stop, Delete, etc.) */
.btn-danger {
  background: var(--terminal-red);
  color: white;
}

/* Secondary Actions */
.btn-secondary {
  background: transparent;
  color: var(--btpc-blue);
  border: 1px solid var(--btpc-blue);
}
```

### Cards and Containers

#### Basic Card
```css
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.card-header {
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 16px;
  margin-bottom: 16px;
}
```

#### Sidebar Navigation
```css
.sidebar {
  width: 280px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-color);
  height: 100vh;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  color: var(--text-secondary);
  text-decoration: none;
  transition: all 150ms ease-out;
}

.nav-item:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--btpc-blue);
  color: white;
  border-left: 4px solid var(--btpc-gold);
}

.nav-icon {
  width: 20px;
  height: 20px;
  margin-right: 12px;
}
```

### Status Indicators

#### Status Dots
```css
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
  margin-right: 8px;
}

.status-active { background: var(--terminal-green); }
.status-inactive { background: var(--terminal-red); }
.status-processing {
  background: var(--terminal-amber);
  animation: pulse 1.5s ease-in-out infinite;
}
.status-unknown { background: var(--text-muted); }
```

#### Badge Components
```css
.badge {
  display: inline-flex;
  align-items: center;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.badge-success {
  background: rgba(72, 187, 120, 0.2);
  color: var(--terminal-green);
}

.badge-warning {
  background: rgba(237, 137, 54, 0.2);
  color: var(--terminal-amber);
}

.badge-error {
  background: rgba(245, 101, 101, 0.2);
  color: var(--terminal-red);
}
```

### Form Elements

#### Input Fields
```css
.input {
  width: 100%;
  padding: 12px 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  color: var(--text-primary);
  font-family: 'Fira Code', monospace;
  font-size: 0.875rem;
  transition: border-color 150ms ease-out;
}

.input:focus {
  outline: none;
  border-color: var(--btpc-blue);
  box-shadow: 0 0 0 3px rgba(26, 54, 93, 0.1);
}

.input::placeholder {
  color: var(--text-muted);
}
```

#### Select Dropdowns
```css
.select {
  position: relative;
  display: inline-block;
  width: 100%;
}

.select select {
  width: 100%;
  padding: 12px 40px 12px 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  color: var(--text-primary);
  font-family: 'Fira Code', monospace;
  appearance: none;
  cursor: pointer;
}
```

## Iconography

### Icon System
- **Style**: Outlined icons with 2px stroke weight
- **Size**: 16px (small), 20px (medium), 24px (large)
- **Color**: Inherit from parent text color
- **Usage**: Consistent icon usage across similar functions

### Common Icons
- **<�** Home/Dashboard
- **=�** Wallet/Money operations
- **�** Mining operations
- **=** Node/Connection operations
- **=�** History/Transactions
- **='** Settings/Configuration
- **=�** Analytics/Charts
- **=** Search/Explore
- **=** Refresh/Reload
- **d** Favorite/Bookmark

## Layout Grids

### Main Grid System
```css
.container {
  display: grid;
  grid-template-columns: 280px 1fr;
  height: 100vh;
  overflow: hidden;
}

.main-content {
  padding: 24px;
  overflow-y: auto;
}
```

### Content Grid
```css
.content-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  margin-bottom: 24px;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}
```

## Animation System

### Transitions
```css
/* Standard transitions */
.transition-default { transition: all 150ms ease-out; }
.transition-slow { transition: all 300ms ease-out; }
.transition-colors { transition: color, background-color, border-color 150ms ease-out; }

/* Hover effects */
.hover-lift:hover { transform: translateY(-2px); }
.hover-scale:hover { transform: scale(1.05); }
```

### Loading States
```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.loading-pulse { animation: pulse 1.5s ease-in-out infinite; }
.loading-spin { animation: spin 1s linear infinite; }
```

## Responsive Design

### Breakpoints
```css
/* Tablet */
@media (max-width: 1023px) {
  .sidebar {
    transform: translateX(-100%);
    position: absolute;
    z-index: 100;
  }

  .sidebar.open {
    transform: translateX(0);
  }
}

/* Mobile */
@media (max-width: 767px) {
  .container {
    grid-template-columns: 1fr;
  }

  .main-content {
    padding: 16px;
  }
}
```

## Data Display Standards

### Transaction and Cryptographic Data

#### Transaction ID Display
```css
.tx-id {
  font-family: 'Fira Code', monospace;
  font-size: 0.8rem;
  color: var(--terminal-cyan);
  background: rgba(56, 178, 172, 0.1);
  padding: 4px 8px;
  border-radius: 4px;
  word-break: break-all;
  user-select: all;
}

/* Truncated display for lists */
.tx-id-short {
  font-family: 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
```

**Formatting Rules**:
- **Full Display**: Show complete transaction ID with copy button
- **Truncated Display**: First 8 + "..." + last 8 characters
- **Always Copyable**: Include one-click copy functionality
- **Monospace Font**: Use Fira Code for all cryptographic data

#### Address Display Standards
```css
.wallet-address {
  font-family: 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--text-muted);
  background: rgba(56, 178, 172, 0.1);
  padding: 4px 8px;
  border-radius: 4px;
  word-break: break-all;
  user-select: all;
}

.address-label {
  display: flex;
  align-items: center;
  gap: 8px;
}

.address-copy-btn {
  cursor: pointer;
  color: var(--terminal-cyan);
  padding: 4px;
}
```

**Display Patterns**:
- **List View**: Truncate to first 8 + last 8 characters
- **Detail View**: Show full address with QR code
- **Copy Button**: Always adjacent to address display
- **Validation Indicator**: Green checkmark for valid, red X for invalid

#### Decimal Precision Standards
```css
/* Financial amounts - 8 decimal places */
.amount {
  font-family: 'Fira Code', monospace;
  font-size: 1rem;
  font-weight: 500;
  color: var(--text-primary);
  letter-spacing: 0.02em;
}

.amount-large {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--btpc-gold);
}

.amount-unit {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-left: 4px;
}
```

**Formatting Rules**:
- Always display 8 decimal places for BTPC amounts
- Use `.` as decimal separator
- Include "BTPC" unit label
- Group thousands with `,` for amounts ≥ 1,000

### Block and Chain Data

#### Block Height Display
```css
.block-height {
  font-family: 'Fira Code', monospace;
  font-size: 0.875rem;
  color: var(--terminal-green);
  font-weight: 500;
}

.block-info-row {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-color);
}
```

#### Hash Display Standards
```css
.hash-display {
  font-family: 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--terminal-cyan);
  background: var(--bg-primary);
  padding: 8px 12px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  overflow-x: auto;
  white-space: nowrap;
}

.hash-label {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 4px;
}
```

## User Guidance Patterns

### Instructional Content

#### Step-by-Step Instructions
```html
<div class="guide-container">
  <div class="guide-step">
    <div class="step-number">1</div>
    <div class="step-content">
      <h3 class="step-title">Action Title</h3>
      <p class="step-description">Clear, action-oriented description</p>
    </div>
  </div>
</div>
```

```css
.guide-container {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 24px;
  margin-bottom: 24px;
}

.guide-step {
  display: flex;
  gap: 16px;
  margin-bottom: 20px;
}

.step-number {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  background: var(--btpc-blue);
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.875rem;
}

.step-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.step-description {
  font-size: 0.875rem;
  color: var(--text-secondary);
  line-height: 1.5;
}
```

**Content Guidelines**:
- Use numbered steps for sequential processes
- Keep each step concise (1-2 sentences)
- Use action verbs at the start of each step
- Include visual confirmation of expected outcomes

### Warning and Information Messages

#### Alert Components
```css
.alert {
  padding: 16px;
  border-radius: 8px;
  border-left: 4px solid;
  margin-bottom: 20px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.alert-icon {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
}

.alert-content {
  flex: 1;
}

.alert-title {
  font-weight: 600;
  font-size: 0.875rem;
  margin-bottom: 4px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.alert-message {
  font-size: 0.875rem;
  line-height: 1.5;
}

/* Alert variants */
.alert-warning {
  background: rgba(237, 137, 54, 0.1);
  border-left-color: var(--terminal-amber);
}

.alert-warning .alert-title {
  color: var(--terminal-amber);
}

.alert-error {
  background: rgba(245, 101, 101, 0.1);
  border-left-color: var(--terminal-red);
}

.alert-error .alert-title {
  color: var(--terminal-red);
}

.alert-info {
  background: rgba(56, 178, 172, 0.1);
  border-left-color: var(--terminal-cyan);
}

.alert-info .alert-title {
  color: var(--terminal-cyan);
}

.alert-success {
  background: rgba(72, 187, 120, 0.1);
  border-left-color: var(--terminal-green);
}

.alert-success .alert-title {
  color: var(--terminal-green);
}
```

**Usage Guidelines**:
- Place warnings prominently before related actions
- Use WARNING/ERROR/INFO/SUCCESS titles
- Capitalize alert titles for emphasis
- Explain implications clearly and concisely

### Technical Operation Displays

#### Command Input Fields
```css
.command-input-group {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.command-label {
  font-family: 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--terminal-green);
  padding: 0 8px;
  font-weight: 500;
}

.command-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-family: 'Fira Code', monospace;
  font-size: 0.875rem;
  padding: 8px;
}

.command-execute-btn {
  background: var(--btpc-blue);
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
}
```

#### Output Display Panels
```css
.output-panel {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 16px;
  margin-top: 12px;
  max-height: 300px;
  overflow-y: auto;
}

.output-line {
  font-family: 'Fira Code', monospace;
  font-size: 0.8rem;
  line-height: 1.6;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.output-line-success {
  color: var(--terminal-green);
}

.output-line-error {
  color: var(--terminal-red);
}

.output-line-info {
  color: var(--terminal-cyan);
}
```

## Documentation and Help Patterns

### In-App Help Content

#### Tooltip Guidelines
```css
.tooltip-trigger {
  position: relative;
  cursor: help;
  border-bottom: 1px dotted var(--text-secondary);
}

.tooltip {
  position: absolute;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 8px 12px;
  font-size: 0.75rem;
  color: var(--text-primary);
  max-width: 200px;
  z-index: 1000;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
}
```

**Content Guidelines**:
- Keep tooltips under 20 words
- Explain technical terms in simple language
- Avoid jargon unless necessary
- Include links to full documentation for complex topics

#### Help Panels
```css
.help-panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 24px;
}

.help-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 12px;
}

.help-content {
  font-size: 0.875rem;
  color: var(--text-secondary);
  line-height: 1.6;
}

.help-link {
  color: var(--terminal-cyan);
  text-decoration: none;
  border-bottom: 1px solid transparent;
  transition: border-color 150ms ease-out;
}

.help-link:hover {
  border-bottom-color: var(--terminal-cyan);
}
```

### Privacy and Security Notices

#### Privacy Warning Pattern
```html
<div class="alert alert-warning">
  <div class="alert-icon">⚠️</div>
  <div class="alert-content">
    <div class="alert-title">Privacy Notice</div>
    <div class="alert-message">
      This operation may reveal transaction details to network observers.
      Consider privacy implications before proceeding.
    </div>
  </div>
</div>
```

**Content Guidelines**:
- Always disclose privacy implications before data-revealing operations
- Use clear, non-technical language
- Explain what information might be exposed
- Provide alternative methods if available

## Implementation Guidelines

### CSS Custom Properties
All colors and spacing should use CSS custom properties for easy theming and maintenance.

### Component Naming
Follow BEM (Block-Element-Modifier) methodology for CSS class naming:
- Block: `.card`
- Element: `.card__header`
- Modifier: `.card--highlighted`

### Accessibility
- Maintain minimum 4.5:1 contrast ratio
- Provide focus indicators for all interactive elements
- Use semantic HTML elements
- Include proper ARIA labels where needed
- Ensure all cryptographic data fields are keyboard-navigable
- Provide screen reader labels for complex data visualizations

### Performance
- Use CSS transforms for animations (not layout properties)
- Minimize repaints and reflows
- Optimize icon usage with sprite sheets or icon fonts
- Lazy-load transaction history for large datasets

### Browser Support
Target modern browsers with CSS Grid and Flexbox support:
- Chrome 70+
- Firefox 65+
- Safari 12+
- Edge 79+

### Content Writing Standards

#### Technical Communication
- **Clarity Over Complexity**: Use simple language to explain technical concepts
- **Action-Oriented**: Start instructions with verbs (e.g., "Enter", "Click", "Verify")
- **Progressive Disclosure**: Show basic information first, advanced details on demand
- **Consistent Terminology**: Use the same terms throughout the application
  - "Transaction ID" not "TxID" or "TXID"
  - "Block Height" not "Block Number"
  - "Wallet Address" not "Address" or "Public Key"

#### Error Messages
```css
.error-message {
  color: var(--terminal-red);
  font-size: 0.875rem;
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
}
```

**Content Guidelines**:
- State what went wrong clearly
- Explain why it happened (if known)
- Provide actionable steps to resolve
- Avoid technical error codes in user-facing messages

**Example Patterns**:
- ❌ "Error: UTXO_NOT_FOUND"
- ✅ "Transaction failed: Insufficient balance. Please check your available funds."

## UI Screen Patterns

### Welcome & Onboarding Flow

#### Language Selection Screen
**Purpose**: Allow users to select their preferred language before entering the wallet

**Layout**:
- Centered modal or card (max-width: 600px)
- BTPC logo at top
- Language dropdown or grid of language options
- Continue button (disabled until selection made)

**Components**:
```html
<div class="welcome-screen">
  <div class="logo-container">
    <img src="btpc-logo.svg" alt="BTPC" class="logo-large" />
    <h1>Welcome to BTPC Wallet</h1>
  </div>
  <div class="language-selector">
    <label for="language">Choose a Language</label>
    <select id="language" class="select">
      <option value="en">English</option>
      <option value="es">Español</option>
      <!-- More languages -->
    </select>
  </div>
  <button class="btn-primary">Continue</button>
</div>
```

#### Wallet Mode Selection
**Purpose**: Allow users to choose between Simple Mode and Advanced Mode

**Options**:
1. **Simple Mode**: Connect to remote node, no blockchain download
2. **Simple Mode (Bootstrap)**: Local node + temporary remote connection
3. **Advanced Mode**: Full control over node settings, mining, advanced features

**Layout Pattern**:
```html
<div class="mode-selection">
  <h2>Choose Wallet Mode</h2>
  <div class="mode-cards">
    <div class="mode-card" data-mode="simple">
      <div class="mode-icon">📱</div>
      <h3>Simple Mode</h3>
      <p>Quick setup, connects to remote node</p>
      <ul class="mode-features">
        <li>✓ No blockchain download</li>
        <li>✓ Basic wallet features</li>
        <li>⚠ Uses remote node (privacy tradeoff)</li>
      </ul>
    </div>
    <div class="mode-card" data-mode="advanced">
      <div class="mode-icon">⚙️</div>
      <h3>Advanced Mode</h3>
      <p>Full control, local or remote node</p>
      <ul class="mode-features">
        <li>✓ All features available</li>
        <li>✓ Mining support</li>
        <li>✓ Maximum privacy options</li>
      </ul>
    </div>
  </div>
</div>
```

**CSS**:
```css
.mode-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
  margin: 24px 0;
}

.mode-card {
  background: var(--bg-secondary);
  border: 2px solid var(--border-color);
  border-radius: 8px;
  padding: 24px;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.mode-card:hover {
  border-color: var(--btpc-blue);
  transform: translateY(-2px);
}

.mode-card.selected {
  border-color: var(--btpc-gold);
  background: rgba(255, 215, 0, 0.05);
}

.mode-icon {
  font-size: 3rem;
  margin-bottom: 16px;
  text-align: center;
}

.mode-features {
  list-style: none;
  padding: 0;
  margin-top: 16px;
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.mode-features li {
  padding: 4px 0;
}
```

### Wallet Creation & Management

#### Create New Wallet Screen
**Purpose**: Guide user through creating a new wallet with mnemonic seed

**Required Elements**:
1. Wallet name input
2. Mnemonic seed display (25 words) with copy button
3. Security warning about seed importance
4. Wallet location selector
5. Password creation fields

**Layout Pattern**:
```html
<div class="wallet-creation">
  <div class="progress-indicator">
    <div class="progress-step active">1. Wallet Details</div>
    <div class="progress-step">2. Secure Your Seed</div>
    <div class="progress-step">3. Add Password</div>
  </div>

  <div class="creation-step" data-step="1">
    <h2>Create New Wallet</h2>

    <div class="form-group">
      <label class="text-label">Wallet Name</label>
      <input type="text" class="input" placeholder="My BTPC Wallet" />
    </div>

    <div class="form-group">
      <label class="text-label">Wallet Location</label>
      <div class="file-selector">
        <input type="text" class="input" readonly value="/home/user/.btpc/wallets" />
        <button class="btn-secondary">Browse</button>
      </div>
    </div>
  </div>

  <div class="creation-step" data-step="2">
    <div class="alert alert-warning">
      <div class="alert-icon">⚠️</div>
      <div class="alert-content">
        <div class="alert-title">CRITICAL: Write Down Your Seed</div>
        <div class="alert-message">
          Your mnemonic seed is the ONLY way to recover your wallet.
          Write it down and store it securely offline. Never share it with anyone.
        </div>
      </div>
    </div>

    <div class="seed-display">
      <div class="seed-grid">
        <span class="seed-word"><span class="word-number">1.</span> abandon</span>
        <span class="seed-word"><span class="word-number">2.</span> ability</span>
        <!-- ... 25 words total -->
      </div>
      <button class="btn-secondary copy-seed-btn">
        <span class="icon">📋</span> Copy Seed to Clipboard
      </button>
    </div>
  </div>

  <div class="creation-step" data-step="3">
    <h3>Add a Password</h3>
    <p>Protect your wallet file with a strong password</p>

    <div class="form-group">
      <label class="text-label">Password</label>
      <input type="password" class="input" />
    </div>

    <div class="form-group">
      <label class="text-label">Confirm Password</label>
      <input type="password" class="input" />
    </div>

    <div class="password-strength">
      <div class="strength-bar">
        <div class="strength-fill" style="width: 60%; background: var(--terminal-amber);"></div>
      </div>
      <span class="strength-label">Password Strength: Medium</span>
    </div>
  </div>

  <div class="action-buttons">
    <button class="btn-secondary">Back</button>
    <button class="btn-primary">Continue</button>
  </div>
</div>
```

**CSS for Seed Display**:
```css
.seed-display {
  background: var(--bg-primary);
  border: 2px solid var(--terminal-amber);
  border-radius: 8px;
  padding: 24px;
  margin: 20px 0;
}

.seed-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 12px;
  margin-bottom: 20px;
}

.seed-word {
  font-family: 'Fira Code', monospace;
  font-size: 0.875rem;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.word-number {
  color: var(--text-muted);
  font-size: 0.75rem;
  min-width: 24px;
}

.copy-seed-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
```

#### Restore Wallet Screen
**Purpose**: Allow users to restore wallet from mnemonic seed or private keys

**Options**:
1. Restore from 25-word mnemonic seed
2. Restore from private keys (view key + spend key)
3. Restore from hardware wallet

**Key Fields**:
- Wallet name
- Mnemonic seed input (25 words) or key inputs
- Restore height (optional, for faster sync)
- Creation date (optional alternative to restore height)
- Wallet location

**Layout Pattern**:
```html
<div class="restore-wallet">
  <h2>Restore Wallet</h2>

  <div class="restore-method-selector">
    <button class="method-btn active" data-method="seed">From Seed</button>
    <button class="method-btn" data-method="keys">From Keys</button>
  </div>

  <div class="restore-form" data-method="seed">
    <div class="form-group">
      <label class="text-label">Wallet Name</label>
      <input type="text" class="input" placeholder="Restored Wallet" />
    </div>

    <div class="form-group">
      <label class="text-label">Mnemonic Seed (25 words)</label>
      <textarea class="input seed-textarea" rows="4"
        placeholder="Enter your 25-word mnemonic seed here..."></textarea>
      <span class="help-text">Separate words with spaces</span>
    </div>

    <div class="form-group optional-group">
      <label class="text-label">
        Restore Height (Optional)
        <span class="tooltip-trigger" data-tooltip="Skip scanning blocks before this height for faster sync">
          ℹ️
        </span>
      </label>
      <input type="number" class="input" placeholder="e.g., 1350000" />
    </div>

    <div class="form-group optional-group">
      <label class="text-label">Creation Date (Optional)</label>
      <input type="date" class="input" />
    </div>
  </div>

  <div class="action-buttons">
    <button class="btn-secondary">Cancel</button>
    <button class="btn-primary">Restore Wallet</button>
  </div>
</div>
```

### Main Dashboard Screen

#### Layout Structure
**Purpose**: Central hub for wallet operations after login

**Sidebar (280px)**:
- Wallet selector dropdown (if multiple wallets)
- Navigation menu items:
  - Dashboard (home icon)
  - Send (send icon)
  - Receive (receive icon)
  - Transactions (history icon)
  - Address Book (contact icon)
  - Mining (mining icon, Advanced mode only)
  - Settings (gear icon)

**Main Content Area**:
- Account overview section (balance, account selector)
- Quick action cards (organized by category)
- System status indicators

**Pattern**:
```html
<div class="dashboard-layout">
  <aside class="sidebar">
    <div class="wallet-selector">
      <select class="select">
        <option>Main Wallet</option>
        <option>Mining Rewards</option>
      </select>
    </div>

    <nav class="nav-menu">
      <a href="#dashboard" class="nav-item active">
        <span class="nav-icon">🏠</span>
        <span class="nav-label">Dashboard</span>
      </a>
      <a href="#send" class="nav-item">
        <span class="nav-icon">↗️</span>
        <span class="nav-label">Send</span>
      </a>
      <a href="#receive" class="nav-item">
        <span class="nav-icon">↙️</span>
        <span class="nav-label">Receive</span>
      </a>
      <a href="#transactions" class="nav-item">
        <span class="nav-icon">📜</span>
        <span class="nav-label">Transactions</span>
      </a>
      <a href="#mining" class="nav-item">
        <span class="nav-icon">⛏️</span>
        <span class="nav-label">Mining</span>
      </a>
      <a href="#settings" class="nav-item">
        <span class="nav-icon">⚙️</span>
        <span class="nav-label">Settings</span>
      </a>
    </nav>
  </aside>

  <main class="main-content">
    <header class="page-header">
      <h1 class="text-h1">Dashboard</h1>
      <div class="sync-status">
        <span class="status-dot status-active"></span>
        <span>Synchronized</span>
      </div>
    </header>

    <section class="account-overview">
      <div class="balance-card">
        <div class="balance-label">Total Balance</div>
        <div class="balance-amount amount-large">1,234.56789012 BTPC</div>
        <div class="balance-details">
          <span>Unlocked: 1,234.56789012 BTPC</span>
          <span>Locked: 0.00000000 BTPC</span>
        </div>
      </div>

      <div class="account-selector">
        <label class="text-label">Active Account</label>
        <select class="select">
          <option>Primary Account</option>
          <option>Account 1</option>
        </select>
        <button class="btn-secondary create-account-btn">+ New Account</button>
      </div>
    </section>

    <section class="quick-actions">
      <div class="content-grid">
        <div class="action-card">
          <h3 class="card-title">Node Management</h3>
          <div class="card-content">
            <div class="status-row">
              <span>Node Status:</span>
              <span class="badge badge-success">Running</span>
            </div>
            <div class="button-group">
              <button class="btn-danger">Stop Node</button>
              <button class="btn-secondary">View Status</button>
            </div>
          </div>
        </div>

        <div class="action-card">
          <h3 class="card-title">Wallet Operations</h3>
          <div class="card-content">
            <div class="button-group-vertical">
              <button class="btn-success">Send BTPC</button>
              <button class="btn-primary">Receive BTPC</button>
              <button class="btn-secondary">Check Balance</button>
              <button class="btn-secondary">Show Address</button>
            </div>
          </div>
        </div>

        <div class="action-card">
          <h3 class="card-title">Mining Operations</h3>
          <div class="card-content">
            <div class="mining-input-group">
              <label>CPU Threads:</label>
              <input type="number" class="input" value="4" min="1" max="16" />
            </div>
            <div class="checkbox-group">
              <label>
                <input type="checkbox" />
                <span>Background Mining</span>
              </label>
            </div>
            <div class="button-group">
              <button class="btn-success">Start Mining</button>
              <button class="btn-danger" disabled>Stop Mining</button>
            </div>
            <div class="mining-status">
              <span class="status-dot status-inactive"></span>
              <span>Status: Stopped</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  </main>
</div>
```

### Send Transaction Screen

**Purpose**: Allow users to send BTPC to another address

**Required Elements**:
1. Destination address input (with validation)
2. Amount input (with max/all button)
3. Transaction priority selector (Advanced mode)
4. Description field (optional)
5. Transaction fee display
6. Confirmation dialog

**Layout Pattern**:
```html
<div class="send-screen">
  <h2 class="text-h2">Send BTPC</h2>

  <div class="send-form">
    <div class="form-group">
      <label class="text-label">Destination Address</label>
      <div class="address-input-group">
        <input type="text" class="input"
          placeholder="btpc1..."
          aria-label="Recipient wallet address" />
        <button class="btn-secondary address-book-btn">
          <span class="icon">📖</span> Address Book
        </button>
      </div>
      <div class="field-validation" data-valid="false">
        <span class="validation-icon">✓</span>
        <span class="validation-message">Address is valid</span>
      </div>
    </div>

    <div class="form-group">
      <label class="text-label">Amount</label>
      <div class="amount-input-group">
        <input type="number" class="input amount-input"
          step="0.00000001"
          placeholder="0.00000000" />
        <span class="amount-unit">BTPC</span>
        <button class="btn-secondary max-btn">Max</button>
      </div>
      <div class="available-balance">
        Available: 1,234.56789012 BTPC
      </div>
    </div>

    <div class="form-group advanced-only">
      <label class="text-label">Transaction Priority</label>
      <select class="select">
        <option value="low">Low (slower, cheaper)</option>
        <option value="medium" selected>Medium (balanced)</option>
        <option value="high">High (faster, more expensive)</option>
      </select>
    </div>

    <div class="form-group">
      <label class="text-label">Description (Optional)</label>
      <input type="text" class="input"
        placeholder="Payment for services" />
    </div>

    <div class="transaction-summary">
      <h3>Transaction Summary</h3>
      <div class="summary-row">
        <span>Amount:</span>
        <span class="amount">10.00000000 BTPC</span>
      </div>
      <div class="summary-row">
        <span>Fee:</span>
        <span class="amount">0.00001234 BTPC</span>
      </div>
      <div class="summary-row total">
        <span>Total:</span>
        <span class="amount">10.00001234 BTPC</span>
      </div>
    </div>

    <div class="action-buttons">
      <button class="btn-secondary">Cancel</button>
      <button class="btn-primary send-btn">Send Transaction</button>
    </div>
  </div>
</div>
```

**CSS for Send Screen**:
```css
.amount-input-group {
  display: flex;
  align-items: center;
  gap: 8px;
  position: relative;
}

.amount-input {
  flex: 1;
  text-align: right;
  font-family: 'Fira Code', monospace;
  font-size: 1.125rem;
  font-weight: 500;
}

.amount-unit {
  position: absolute;
  right: 80px;
  color: var(--text-secondary);
  font-size: 0.875rem;
  pointer-events: none;
}

.max-btn {
  min-width: 60px;
}

.available-balance {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 4px;
}

.transaction-summary {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 20px;
  margin: 24px 0;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-color);
}

.summary-row.total {
  border-bottom: none;
  margin-top: 8px;
  font-weight: 600;
  font-size: 1.125rem;
  color: var(--btpc-gold);
}

.field-validation {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  font-size: 0.75rem;
}

.field-validation[data-valid="true"] {
  color: var(--terminal-green);
}

.field-validation[data-valid="false"] {
  color: var(--terminal-red);
}
```

### Receive Screen

**Purpose**: Display receiving address(es) and generate payment requests

**Required Elements**:
1. Primary address display with QR code
2. Subaddress list (Advanced mode)
3. Amount input for payment request
4. Payment URL generator
5. Copy buttons for address/QR/URL

**Layout Pattern**:
```html
<div class="receive-screen">
  <h2 class="text-h2">Receive BTPC</h2>

  <div class="receive-container">
    <div class="primary-address-section">
      <h3 class="text-h3">Primary Address</h3>

      <div class="qr-display">
        <img src="qr-code.png" alt="QR Code" class="qr-image" />
        <button class="btn-secondary save-qr-btn">Save QR Code</button>
      </div>

      <div class="address-display-group">
        <label class="text-label">Your BTPC Address</label>
        <div class="address-display">
          <code class="wallet-address">btpc1abc123def456...xyz789</code>
          <button class="btn-secondary copy-btn" data-copy="address">
            <span class="icon">📋</span> Copy
          </button>
        </div>
      </div>

      <div class="payment-request-section">
        <h4>Create Payment Request</h4>

        <div class="form-group">
          <label class="text-label">Amount (Optional)</label>
          <input type="number" class="input amount-input"
            step="0.00000001"
            placeholder="0.00000000" />
        </div>

        <div class="form-group">
          <label class="text-label">Payment URL</label>
          <div class="url-display">
            <code class="payment-url">btpc:btpc1abc...?amount=10</code>
            <button class="btn-secondary copy-btn" data-copy="url">
              <span class="icon">📋</span> Copy
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="subaddress-section advanced-only">
      <h3 class="text-h3">Subaddresses</h3>
      <p class="help-text">Use different addresses for better privacy</p>

      <button class="btn-primary create-subaddress-btn">
        + Create New Subaddress
      </button>

      <div class="subaddress-list">
        <div class="subaddress-item">
          <div class="subaddress-label">Payment #1</div>
          <div class="subaddress-value">
            <code>btpc1sub...addr</code>
            <button class="btn-secondary copy-btn">📋</button>
          </div>
          <button class="btn-secondary edit-label-btn">Edit Label</button>
        </div>
      </div>
    </div>
  </div>
</div>
```

### Transaction History Screen

**Purpose**: Display list of all transactions with search and filtering

**Required Elements**:
1. Search input (filter by address, txid, description)
2. Date range filter
3. Transaction type filter (sent/received/all)
4. Transaction list with pagination
5. Transaction details modal/panel

**Layout Pattern**:
```html
<div class="transactions-screen">
  <h2 class="text-h2">Transaction History</h2>

  <div class="transaction-controls">
    <div class="search-filter-row">
      <div class="search-input-group">
        <input type="text" class="input search-input"
          placeholder="Search by address, TX ID, or description..." />
        <button class="btn-secondary search-btn">🔍</button>
      </div>

      <div class="filter-group">
        <select class="select filter-select">
          <option value="all">All Transactions</option>
          <option value="sent">Sent</option>
          <option value="received">Received</option>
        </select>

        <div class="date-range-filter">
          <input type="date" class="input" placeholder="From" />
          <span>—</span>
          <input type="date" class="input" placeholder="To" />
        </div>
      </div>
    </div>
  </div>

  <div class="transaction-list">
    <div class="transaction-item" data-type="received">
      <div class="tx-icon">↙️</div>
      <div class="tx-details">
        <div class="tx-header">
          <span class="tx-type badge badge-success">Received</span>
          <span class="tx-date">2025-09-28 14:32:15</span>
        </div>
        <div class="tx-amount-row">
          <span class="amount positive">+10.50000000 BTPC</span>
        </div>
        <div class="tx-id-row">
          <span class="text-label">TX ID:</span>
          <code class="tx-id-short">abc12345...xyz789</code>
          <button class="btn-secondary copy-btn">📋</button>
        </div>
        <div class="tx-description">Mining rewards</div>
      </div>
      <button class="btn-secondary tx-details-btn">Details</button>
    </div>

    <div class="transaction-item" data-type="sent">
      <div class="tx-icon">↗️</div>
      <div class="tx-details">
        <div class="tx-header">
          <span class="tx-type badge badge-warning">Sent</span>
          <span class="tx-date">2025-09-27 10:15:42</span>
        </div>
        <div class="tx-amount-row">
          <span class="amount negative">-5.00000000 BTPC</span>
          <span class="tx-fee">Fee: 0.00001234 BTPC</span>
        </div>
        <div class="tx-id-row">
          <span class="text-label">TX ID:</span>
          <code class="tx-id-short">def45678...uvw012</code>
          <button class="btn-secondary copy-btn">📋</button>
        </div>
      </div>
      <button class="btn-secondary tx-details-btn">Details</button>
    </div>
  </div>

  <div class="pagination">
    <button class="btn-secondary" disabled>← Previous</button>
    <span class="page-info">Page 1 of 10</span>
    <button class="btn-secondary">Next →</button>
  </div>
</div>
```

**CSS for Transaction List**:
```css
.transaction-list {
  margin-top: 24px;
}

.transaction-item {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 16px;
  transition: all 150ms ease-out;
}

.transaction-item:hover {
  border-color: var(--btpc-blue);
  transform: translateX(4px);
}

.tx-icon {
  font-size: 1.5rem;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
  border-radius: 50%;
}

.tx-details {
  flex: 1;
}

.tx-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.tx-amount-row {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 6px;
}

.amount {
  font-family: 'Fira Code', monospace;
  font-size: 1.125rem;
  font-weight: 600;
}

.amount.positive {
  color: var(--terminal-green);
}

.amount.negative {
  color: var(--text-primary);
}

.tx-fee {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.tx-description {
  font-size: 0.875rem;
  color: var(--text-secondary);
  font-style: italic;
  margin-top: 4px;
}
```

### Settings Screen

**Purpose**: Configure wallet, interface, node, and application settings

**Sections** (Tab-based navigation):
1. **Wallet Settings**: Password, view-only wallet, rescan, merchant mode
2. **Interface Settings**: Theme, language, balance visibility, auto-save
3. **Node Settings**: Local node configuration, blockchain location
4. **Remote Node**: Remote node address, port, authentication
5. **Log**: Debug log viewer and settings
6. **Info**: Version info, wallet path, restore height
7. **Seed & Keys**: Display mnemonic seed and private keys (password protected)

**Layout Pattern**:
```html
<div class="settings-screen">
  <h2 class="text-h2">Settings</h2>

  <div class="settings-layout">
    <nav class="settings-tabs">
      <button class="tab-btn active" data-tab="wallet">Wallet</button>
      <button class="tab-btn" data-tab="interface">Interface</button>
      <button class="tab-btn" data-tab="node">Local Node</button>
      <button class="tab-btn" data-tab="remote">Remote Node</button>
      <button class="tab-btn" data-tab="log">Log</button>
      <button class="tab-btn" data-tab="info">Info</button>
      <button class="tab-btn" data-tab="keys">Seed & Keys</button>
    </nav>

    <div class="settings-content">
      <div class="settings-panel active" data-panel="wallet">
        <h3 class="text-h3">Wallet Settings</h3>

        <div class="settings-group">
          <button class="btn-secondary">Close This Wallet</button>
          <p class="help-text">Safely close the current wallet and return to wallet selection</p>
        </div>

        <div class="settings-group">
          <button class="btn-secondary">Create View-Only Wallet</button>
          <p class="help-text">Generate a wallet that can view balance but not spend funds</p>
        </div>

        <div class="settings-group">
          <button class="btn-primary">Show Seed & Keys</button>
          <p class="help-text">View your mnemonic seed and private keys</p>
        </div>

        <div class="settings-group">
          <button class="btn-warning">Rescan Wallet Balance</button>
          <p class="help-text">Rescan all spent outputs (use if balance appears incorrect)</p>
        </div>

        <div class="settings-group">
          <button class="btn-secondary">Change Wallet Password</button>
          <p class="help-text">Update the password that protects your wallet file</p>
        </div>
      </div>

      <div class="settings-panel" data-panel="interface">
        <h3 class="text-h3">Interface Settings</h3>

        <div class="settings-group">
          <label class="setting-row">
            <span class="setting-label">Light Theme</span>
            <input type="checkbox" class="toggle" />
          </label>
          <p class="help-text">Switch between light and dark color schemes</p>
        </div>

        <div class="settings-group">
          <label class="setting-row">
            <span class="setting-label">Hide Balance</span>
            <input type="checkbox" class="toggle" />
          </label>
          <p class="help-text">Display balance as ?.?? for privacy</p>
        </div>

        <div class="settings-group">
          <label class="setting-row">
            <span class="setting-label">Ask for Password Before Sending</span>
            <input type="checkbox" class="toggle" checked />
          </label>
          <p class="help-text">Require password confirmation before transactions</p>
        </div>

        <div class="settings-group">
          <label class="setting-row">
            <span class="setting-label">Enable Auto-Save</span>
            <input type="checkbox" class="toggle" checked />
          </label>
          <p class="help-text">Automatically save wallet file after changes</p>
        </div>

        <div class="settings-group">
          <label class="setting-row">
            <span class="setting-label">Lock Wallet on Inactivity</span>
            <input type="number" class="input small-input" value="10" min="0" />
            <span>minutes</span>
          </label>
          <p class="help-text">Auto-lock wallet after period of inactivity (0 to disable)</p>
        </div>
      </div>

      <div class="settings-panel" data-panel="keys">
        <div class="alert alert-warning">
          <div class="alert-icon">⚠️</div>
          <div class="alert-content">
            <div class="alert-title">SECURITY WARNING</div>
            <div class="alert-message">
              NEVER share your mnemonic seed or private spend key with anyone.
              Anyone with access to these can steal your funds. Only share your
              public keys or view-only information when necessary.
            </div>
          </div>
        </div>

        <button class="btn-primary show-seed-btn">Show Seed & Keys</button>

        <div class="keys-display hidden" data-protected="true">
          <div class="key-group">
            <label class="text-label">Mnemonic Seed</label>
            <div class="seed-display-compact">
              <code class="seed-text">abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual adapt</code>
              <button class="btn-secondary copy-btn">📋 Copy</button>
            </div>
          </div>

          <div class="key-group">
            <label class="text-label">Primary Address</label>
            <div class="key-value-row">
              <code>btpc1abc123def456...</code>
              <button class="btn-secondary copy-btn">📋</button>
            </div>
          </div>

          <div class="key-group">
            <label class="text-label">Secret View Key</label>
            <div class="key-value-row">
              <code>a1b2c3d4e5f6...</code>
              <button class="btn-secondary copy-btn">📋</button>
            </div>
          </div>

          <div class="key-group critical">
            <label class="text-label">Secret Spend Key ⚠️</label>
            <div class="key-value-row">
              <code>x9y8z7w6v5...</code>
              <button class="btn-secondary copy-btn">📋</button>
            </div>
            <p class="help-text danger">DO NOT SHARE THIS KEY</p>
          </div>

          <div class="key-group">
            <label class="text-label">Public View Key</label>
            <div class="key-value-row">
              <code>pub123abc...</code>
              <button class="btn-secondary copy-btn">📋</button>
            </div>
          </div>

          <div class="key-group">
            <label class="text-label">Public Spend Key</label>
            <div class="key-value-row">
              <code>pub456def...</code>
              <button class="btn-secondary copy-btn">📋</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```

**CSS for Settings**:
```css
.settings-layout {
  display: grid;
  grid-template-columns: 200px 1fr;
  gap: 24px;
  margin-top: 24px;
}

.settings-tabs {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tab-btn {
  background: transparent;
  border: none;
  padding: 12px 16px;
  text-align: left;
  color: var(--text-secondary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.tab-btn:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.tab-btn.active {
  background: var(--btpc-blue);
  color: white;
}

.settings-panel {
  display: none;
}

.settings-panel.active {
  display: block;
}

.settings-group {
  margin-bottom: 32px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--border-color);
}

.settings-group:last-child {
  border-bottom: none;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  cursor: pointer;
}

.setting-label {
  font-weight: 500;
  color: var(--text-primary);
}

.toggle {
  width: 48px;
  height: 24px;
  appearance: none;
  background: var(--bg-tertiary);
  border-radius: 12px;
  position: relative;
  cursor: pointer;
  transition: background 150ms ease-out;
}

.toggle:checked {
  background: var(--terminal-green);
}

.toggle::after {
  content: '';
  position: absolute;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  top: 2px;
  left: 2px;
  transition: transform 150ms ease-out;
}

.toggle:checked::after {
  transform: translateX(24px);
}

.key-group {
  margin-bottom: 24px;
  padding: 16px;
  background: var(--bg-primary);
  border-radius: 8px;
  border: 1px solid var(--border-color);
}

.key-group.critical {
  border-color: var(--terminal-red);
  background: rgba(245, 101, 101, 0.05);
}

.key-value-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

.key-value-row code {
  flex: 1;
  font-family: 'Fira Code', monospace;
  font-size: 0.75rem;
  background: var(--bg-secondary);
  padding: 8px 12px;
  border-radius: 4px;
  overflow-x: auto;
}

.help-text.danger {
  color: var(--terminal-red);
  font-weight: 600;
  margin-top: 8px;
}
```

## Accessibility Requirements

### Screen Reader Support
- All interactive elements must have descriptive `aria-label` attributes
- Form inputs must be associated with `<label>` elements
- Status updates must use `aria-live` regions
- Navigation menus must use proper ARIA roles

### Keyboard Navigation
- All interactive elements must be keyboard accessible
- Logical tab order (left to right, top to bottom)
- Visible focus indicators (outline or border change)
- Escape key closes modals and dialogs
- Enter/Space activates buttons and selections

### Color Contrast
- Minimum 4.5:1 contrast ratio for normal text
- Minimum 3:1 contrast ratio for large text (18px+)
- Do not rely solely on color to convey information
- Provide text labels alongside color-coded status indicators

## Responsive Behavior

### Mobile Considerations (< 768px)
- Sidebar collapses to hamburger menu
- Cards stack vertically
- QR codes remain visible but scale down
- Transaction lists show condensed view
- Action buttons become full-width

### Tablet Considerations (768px - 1023px)
- Sidebar can be toggled with overlay
- Two-column card layout where possible
- Maintain readable font sizes
- Preserve all functionality from desktop

### Desktop Optimization (1024px+)
- Full sidebar always visible
- Multi-column layouts for efficiency
- Hover states for enhanced interactions
- Larger QR codes and visual elements