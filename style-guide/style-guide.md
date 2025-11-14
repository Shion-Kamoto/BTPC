# BTPC Desktop Application Style Guide
*Based on Monero GUI Design System*

## Overview
This style guide defines the visual design language for the BTPC desktop application, following the proven patterns established by the Monero GUI wallet. The design emphasizes security, clarity, and professional cryptocurrency management.

---

## Color Palette

### Primary Brand Colors
```css
--monero-orange: #FF6600;        /* Primary brand accent - used for logo, CTA buttons */
--monero-gray: #4C4C4C;          /* Secondary brand - logo bottom half */
--monero-white: #FFFFFF;         /* Logo M symbol */
```

### Background Colors
```css
--bg-primary: #2C2C2C;           /* Main application background */
--bg-sidebar: #6B5547;           /* Left sidebar background (brown/tan) */
--bg-card: #3D3D3D;              /* Card and panel backgrounds */
--bg-input: #1A1A1A;             /* Input field backgrounds */
--bg-hover: #4A4A4A;             /* Hover state for interactive elements */
--bg-active: #FF6600;            /* Active navigation item (orange) */
```

### Text Colors
```css
--text-primary: #FFFFFF;         /* Primary text, headings */
--text-secondary: #CCCCCC;       /* Secondary text, descriptions */
--text-muted: #999999;           /* Disabled text, placeholders */
--text-orange: #FF9955;          /* Orange text for labels (e.g., "Account #1") */
--text-link: #FF6600;            /* Clickable links */
```

### Status Colors
```css
--status-success: #4CAF50;       /* Green - Received transactions, synced */
--status-warning: #FF9800;       /* Orange/Amber - Syncing, processing */
--status-error: #F44336;         /* Red - Errors, failed operations */
--status-sent: #FF6600;          /* Orange - Sent transactions */
```

### UI Element Colors
```css
--border-subtle: #555555;        /* Subtle borders between elements */
--border-input: #666666;         /* Input field borders */
--border-active: #FF6600;        /* Active/focused element borders */
--shadow-card: rgba(0,0,0,0.3);  /* Card shadow */
```

---

## Typography

### Font Family
```css
font-family: system-ui, -apple-system, 'Segoe UI', Arial, sans-serif;
```
**Note**: Monero GUI uses system default sans-serif fonts, NOT monospace. This ensures:
- Native OS integration and performance
- Professional, modern appearance
- Better readability for cryptocurrency addresses and amounts

### Font Sizes
```css
--text-xs: 11px;                 /* Timestamps, metadata */
--text-sm: 12px;                 /* Labels, secondary text */
--text-base: 14px;               /* Body text, form inputs */
--text-lg: 16px;                 /* Emphasized text */
--text-xl: 18px;                 /* Section headers */
--text-2xl: 24px;                /* Large numbers (balances) */
--text-3xl: 32px;                /* Page titles */
```

### Font Weights
```css
--weight-normal: 400;
--weight-medium: 500;
--weight-semibold: 600;
--weight-bold: 700;
```

### Typography Usage
- **Page Titles**: 32px, weight 600, white
- **Section Headers**: 18px, weight 600, white
- **Body Text**: 14px, weight 400, #CCCCCC
- **Labels**: 12px, weight 400, #999999 or orange (#FF9955)
- **Balances**: 24px, weight 700, white
- **Addresses**: 14px, monospace fallback allowed, white
- **Button Text**: 14px, weight 600, white

---

## Layout Structure

### Application Grid
```
┌─────────────────────────────────────────────────────────┐
│  Titlebar (#2C2C2C)           [MONERO]     [─][□][×]  │
├──────────────┬──────────────────────────────────────────┤
│              │                                          │
│   Sidebar    │         Main Content Area                │
│   280px      │                                          │
│   (#6B5547)  │         (#2C2C2C)                        │
│              │                                          │
│              │                                          │
│              │                                          │
│              ├──────────────────────────────────────────┤
│              │   Footer Status Bar                      │
│              │   Network status, sync progress          │
└──────────────┴──────────────────────────────────────────┘
```

### Sidebar (Left Panel)
- **Width**: 280px fixed
- **Background**: `#6B5547` (warm brown/tan color)
- **Structure**:
  1. Account Card (top) - 280px × ~160px
  2. Navigation Menu Items
  3. Footer with sync status

### Main Content Area
- **Background**: `#2C2C2C` (dark gray)
- **Padding**: 24px on all sides
- **Max Width**: Fills remaining space after sidebar

---

## Components

### 1. Account Card (Sidebar Header)
Located at the top of the sidebar, displays current account information.

**Specifications**:
- Background: `linear-gradient(135deg, #8B7355 0%, #6B5547 100%)`
- Padding: 20px
- Border-radius: 8px
- Contains:
  - Monero logo (48px circle with orange M)
  - "MONERO" text
  - Account label (e.g., "Balance (#0 - Main account)")
  - Balance amount (large, white text)
  - Unlocked balance (smaller, secondary text)

**Example**:
```
┌────────────────────────┐
│  [M] MONERO            │
│                        │
│  Balance (#0 - Main)   │
│  4370.852667263602     │
│                        │
│  Unlocked (~94 min)    │
│  4285.583187378335     │
└────────────────────────┘
```

### 2. Navigation Items
**Dimensions**:
- Height: 48px
- Padding: 12px 16px
- Full width of sidebar (280px)

**States**:
- **Default**: Transparent background, white text with icon
- **Hover**: Background `rgba(255,255,255,0.1)`
- **Active**: Background `#FF6600` (orange), white text, subtle left border accent

**Structure**:
- Status dot (8px circle) - left aligned
- Item label - 16px left margin from dot
- Chevron (›) - right aligned

**Color Indicators**:
- Blue dot: Account section
- Orange dot: Send section
- Green dot: Receive section (active)
- Purple dot: Advanced section
- Teal dot: Settings section

### 3. Primary Buttons
**Default (Orange CTA)**:
```css
background: #FF6600;
color: #FFFFFF;
border: none;
border-radius: 4px;
padding: 12px 24px;
font-size: 14px;
font-weight: 600;
min-height: 40px;
```

**Secondary (Gray)**:
```css
background: #666666;
color: #FFFFFF;
border: none;
border-radius: 4px;
padding: 12px 24px;
```

**Hover State**: Lighten background by 10%
**Active/Pressed**: Darken background by 10%

### 4. Input Fields
```css
background: #1A1A1A;
border: 1px solid #666666;
border-radius: 4px;
color: #FFFFFF;
padding: 12px 16px;
font-size: 14px;
min-height: 44px;
```

**Focus State**:
```css
border-color: #FF6600;
outline: 2px solid rgba(255, 102, 0, 0.3);
outline-offset: 2px;
```

### 5. Dropdown/Select
- Same styling as input fields
- Right-aligned chevron (▼) indicator
- Dropdown menu: `background: #3D3D3D`, `border: 1px solid #666666`

### 6. Checkboxes
```css
width: 18px;
height: 18px;
border: 2px solid #666666;
border-radius: 3px;
background: transparent;
```

**Checked State**:
```css
background: #FF6600;
border-color: #FF6600;
```
White checkmark (✓) centered

### 7. Progress Bars (Sync Status)
Located in sidebar footer:
```css
width: 100%;
height: 4px;
background: rgba(255,255,255,0.2);
border-radius: 2px;
```

**Progress Fill**:
```css
background: #FF6600;
height: 100%;
border-radius: 2px;
transition: width 0.3s ease;
```

### 8. Transaction List Items
**Row Height**: 56px minimum (expandable)
**Background**: Alternating `#2C2C2C` and `#323232`

**Structure** (left to right):
1. Status dot (left, 8px) - Green for received, Orange for sent
2. Transaction type & amount (bold)
3. Destination/Source (secondary text)
4. Date/confirmations (right aligned)
5. Expand chevron (›) - far right

### 9. QR Code Display
- White QR code on `#FFFFFF` background
- Container: `background: #FFFFFF`, `padding: 16px`, `border-radius: 8px`
- Action buttons below: Orange buttons for copy/save

### 10. Tab Navigation
Horizontal tabs (e.g., Wallet | Interface | Node | Log | Info):
```css
background: transparent;
border: none;
border-bottom: 3px solid transparent;
color: #999999;
padding: 12px 24px;
font-size: 14px;
font-weight: 600;
```

**Active Tab**:
```css
border-bottom-color: #FF6600;
color: #FFFFFF;
```

---

## Logo Specifications

### Monero Logo Mark
The Monero logo is a two-tone circle with an "M" symbol:

**Dimensions**: 48px × 48px (sidebar), 32px × 32px (titlebar)

**Colors**:
- Top half (background): `#FF6600` (Orange)
- Bottom half (background): `#4C4C4C` (Dark Gray)
- "M" symbol: `#FFFFFF` (White)

**For BTPC**: Replace with quantum-themed logo using same dimensional specifications

---

## Spacing System

```css
--space-xs: 4px;
--space-sm: 8px;
--space-md: 12px;
--space-lg: 16px;
--space-xl: 20px;
--space-2xl: 24px;
--space-3xl: 32px;
--space-4xl: 48px;
```

**Common Applications**:
- Card padding: 20px (`--space-xl`)
- Section spacing: 24px (`--space-2xl`)
- Element margins: 12px (`--space-md`)
- Icon spacing: 12px between icon and text

---

## Border Radius

```css
--radius-sm: 3px;      /* Checkboxes */
--radius-md: 4px;      /* Buttons, inputs */
--radius-lg: 8px;      /* Cards, panels */
--radius-full: 50%;    /* Status dots, circular badges */
```

---

## Shadows

```css
--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
--shadow-md: 0 2px 8px rgba(0, 0, 0, 0.4);
--shadow-lg: 0 8px 16px rgba(0, 0, 0, 0.5);
```

**Usage**:
- Cards: `--shadow-md`
- Modals/Dialogs: `--shadow-lg`
- Dropdowns: `--shadow-md`

---

## Accessibility

### Color Contrast
All text meets WCAG 2.1 AA standards:
- White (#FFFFFF) on dark backgrounds (#2C2C2C): 12.63:1 ✓
- Orange (#FF6600) on dark backgrounds (#2C2C2C): 4.89:1 ✓
- Secondary text (#CCCCCC) on dark backgrounds: 9.73:1 ✓

### Focus Indicators
All interactive elements must have visible focus states:
```css
outline: 2px solid #FF6600;
outline-offset: 2px;
```

### Keyboard Navigation
- Tab order follows visual hierarchy (top to bottom, left to right)
- All buttons and inputs keyboard accessible
- Escape key closes modals and dropdowns

---

## Animation Guidelines

### Transitions
```css
--transition-fast: 150ms ease-out;
--transition-base: 200ms ease-out;
--transition-slow: 300ms ease-out;
```

**Common Uses**:
- Button hover: `background-color 150ms ease-out`
- Navigation active state: `background-color 200ms ease-out`
- Modal appearance: `opacity 200ms ease-out, transform 200ms ease-out`
- Progress bar: `width 300ms ease-out`

### Loading States
- Spinner: Rotating orange circle (24px diameter)
- Skeleton: Pulsing gray placeholders matching content dimensions
- Progress bar: Animated orange fill from left to right

---

## Responsive Behavior

### Minimum Window Size
- Width: 960px
- Height: 600px

### Breakpoints
- Desktop (default): 960px+
- Sidebar remains fixed at 280px
- Main content scales fluidly

**Note**: Monero GUI is desktop-focused with fixed minimum dimensions.

---

## Data Display Formats

### Cryptocurrency Amounts
- **Full precision**: Display up to 12 decimal places
- **Font**: Regular sans-serif (NOT monospace in main display)
- **Color**: White (#FFFFFF) for balances
- **Example**: `4370.852667263602`

### Addresses
- **Format**: First 8 chars... middle ellipsis ...last 6 chars
- **Full address on hover**: Tooltip with complete address
- **Copy button**: Small icon next to address
- **Example**: `55LTR8...yJsXvt`

### Transaction IDs
- **Format**: First 12 chars... ellipsis
- **Click to reveal**: Expands to show full ID
- **Example**: `86adf3f474d0d...`

### Dates/Times
- **Relative**: "3 minute(s) ago", "134 day(s) ago"
- **Absolute on hover**: Full timestamp tooltip
- **Format**: Human-readable with units

### Confirmations
- **Format**: `4/10` (current/required)
- **Color**: Orange while pending, green when confirmed

---

## File Naming Conventions

### Screenshots
Use descriptive prefixes for theme variants:
- `black_*.png` - Dark theme screenshots
- `wizard_*.png` - Setup wizard screens
- `{feature}_*.png` - Feature-specific screens

### Assets
- `monero-logo-1280.png` - Full logo
- `monero-symbol-480.png` - Symbol only
- Use descriptive, lowercase names with hyphens

---

## Implementation Notes

### CSS Custom Properties
Define all colors, spacing, and sizes as CSS variables in `:root`:

```css
:root {
  /* Colors */
  --monero-orange: #FF6600;
  --bg-primary: #2C2C2C;
  --bg-sidebar: #6B5547;

  /* Typography */
  --text-base: 14px;
  --weight-semibold: 600;

  /* Spacing */
  --space-lg: 16px;

  /* Timing */
  --transition-base: 200ms ease-out;
}
```

### Component Classes
Use semantic, reusable class names:
- `.account-card` - Sidebar account display
- `.nav-item` - Navigation menu item
- `.nav-item--active` - Active state modifier
- `.btn-primary` - Primary orange button
- `.btn-secondary` - Secondary gray button
- `.tx-item` - Transaction list item
- `.status-dot` - Colored status indicator

---

## Conclusion

This style guide ensures visual consistency with the proven Monero GUI design while allowing customization for BTPC branding. Key takeaways:

1. **Orange (#FF6600)** is the primary brand accent
2. **Warm brown sidebar (#6B5547)** provides visual hierarchy
3. **System fonts** ensure native performance and readability
4. **280px sidebar** is fixed, content area scales
5. **48px navigation items** ensure touch-friendly targets
6. **Dark theme throughout** with carefully chosen contrast ratios

All components should maintain these specifications for a cohesive, professional cryptocurrency wallet experience.