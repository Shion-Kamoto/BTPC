# Data Model: Fix Non-Functional Sub-Tabs

**Feature**: 004-fix-non-functional
**Date**: 2025-10-25
**Purpose**: Define entities and their relationships for tab switching functionality

---

## 1. Entity Overview

This feature involves **3 primary entities** for managing tab state and behavior:

1. **TabState** - Represents the active tab selection for a specific page
2. **TabButton** - Interactive UI element for tab navigation
3. **TabContent** - Content section displayed when corresponding tab is active

**Entity Relationship**:
```
Page (Settings/Transactions/Mining)
  ├── TabState (1) - Active tab ID stored in localStorage
  ├── TabButtons (n) - Multiple clickable tab buttons
  │     ├── One TabButton.active = true at a time
  │     └── Each TabButton has data-tab attribute matching TabContent ID
  └── TabContents (n) - Multiple content sections
        ├── One TabContent.active = true at a time
        └── Each TabContent ID matches TabButton data-tab attribute
```

**Cardinality**:
- Each Page has **exactly 1** TabState (current active tab)
- Each Page has **n** TabButtons (2-4 tabs depending on page)
- Each Page has **n** TabContents (matching number of buttons)
- Each TabButton controls **exactly 1** TabContent (1:1 mapping)

---

## 2. Entity: TabState

### Description
Represents which sub-tab is currently active on a specific page. This is **not an object in memory** but rather a **stored value in localStorage** representing user preference.

### Properties

| Property | Type | Required | Description | Validation |
|----------|------|----------|-------------|------------|
| `page` | string | Yes | Page identifier (`'settings'`, `'transactions'`, `'mining'`) | Enum: settings, transactions, mining |
| `activeTabId` | string | Yes | ID of currently active tab (e.g., `'node'`, `'receive'`, `'configure'`) | Must match existing tab ID for that page |

### Storage Location
**localStorage** with key pattern: `btpc_active_tab_{page}`

**Examples**:
```javascript
localStorage.getItem('btpc_active_tab_settings')      // Returns: "node"
localStorage.getItem('btpc_active_tab_transactions')  // Returns: "history"
localStorage.getItem('btpc_active_tab_mining')        // Returns: "configure"
```

### Valid Values Per Page

**Settings Page**:
- Valid activeTabIds: `"network"`, `"node"`, `"application"`, `"security"`
- Default: `"node"` (first tab, but currently "network" is leftmost in HTML)

**Transactions Page**:
- Valid activeTabIds: `"receive"`, `"history"`, `"addressbook"`
- Default: `"receive"` (first tab)

**Mining Page**:
- Valid activeTabIds: `"configure"`, `"history"`
- Default: `"configure"` (first tab)

### State Persistence Rules

**FR-009**: System MUST persist active tab selection in browser localStorage
**FR-010**: System MUST restore last active tab when user returns to page during same session
**FR-011**: System MUST restore last active tab when user refreshes browser or reopens app
**FR-012**: System MUST default to first tab when no previous selection exists

**Persistence Logic**:
```javascript
// Write
function saveTabState(page, tabId) {
    localStorage.setItem(`btpc_active_tab_${page}`, tabId);
}

// Read
function loadTabState(page, defaultTabId) {
    return localStorage.getItem(`btpc_active_tab_${page}`) || defaultTabId;
}
```

### Edge Cases

| Scenario | Behavior | Requirement |
|----------|----------|-------------|
| localStorage disabled | Return defaultTabId, log warning | Graceful degradation (Edge Case 1 from spec) |
| Invalid tabId in storage | Return defaultTabId, clear invalid entry | Data integrity |
| First-time user (no entry) | Return defaultTabId | FR-012 |
| Page renamed/restructured | Clear old key, use defaultTabId | Future-proofing |

### Multi-Page Isolation

**FR-017**: System MUST maintain separate active tab state for Settings, Transactions, Mining
**FR-018**: System MUST NOT allow tab state from one page to affect other pages
**FR-019**: System MUST persist each page's tab state independently with unique keys

**Isolation Mechanism**: Each page has unique localStorage key (`btpc_active_tab_{page}`), no shared state.

---

## 3. Entity: TabButton

### Description
Interactive UI element representing a sub-tab option. Responds to mouse clicks and keyboard input to switch active tab.

### DOM Structure

```html
<button
    class="tab-btn active"
    role="tab"
    id="tab-btn-node"
    data-tab="node"
    aria-selected="true"
    aria-controls="tab-panel-node"
    tabindex="0"
    onclick="switchTab('node')"
>
    Node
</button>
```

### Properties (DOM Attributes)

| Property | Type | Required | Description | Requirement |
|----------|------|----------|-------------|-------------|
| `class` | string | Yes | CSS class (`.tab-btn`, `.tab-btn.active` when active) | FR-002, FR-003 |
| `role` | string | Yes | ARIA role `"tab"` | NFR-007 |
| `id` | string | Yes | Unique button identifier (`tab-btn-{tabId}`) | ARIA pattern |
| `data-tab` | string | Yes | Tab identifier matching content ID (`"node"`, `"receive"`, etc.) | FR-001 |
| `aria-selected` | boolean | Yes | `"true"` if active, `"false"` otherwise | NFR-007 |
| `aria-controls` | string | Yes | ID of controlled panel (`tab-panel-{tabId}`) | NFR-007 |
| `tabindex` | number | Yes | `0` if active (focusable), `-1` if inactive (skip in tab order) | FR-013, NFR-008 |
| `onclick` | string | Legacy | Inline handler (will be replaced by event delegation) | FR-001 |

### Visual States

| State | CSS Class | ARIA Attribute | Requirement |
|-------|-----------|----------------|-------------|
| **Active** | `.tab-btn.active` | `aria-selected="true"`, `tabindex="0"` | FR-002 |
| **Inactive** | `.tab-btn` | `aria-selected="false"`, `tabindex="-1"` | FR-003 |
| **Hover** | `.tab-btn:hover` | N/A | FR-004 |
| **Focused (keyboard)** | `.tab-btn:focus` | N/A | FR-016 |

### Behavior Rules

**FR-001**: Must respond to mouse clicks on all sub-tab buttons
**FR-004**: Must provide hover feedback when user hovers over inactive tabs
**FR-013**: Must allow keyboard focus via Tab key navigation
**FR-014**: Must activate when user presses Enter or Space key
**FR-015**: Must support Arrow Left/Right navigation between tabs
**FR-016**: Must show visible focus indicator when focused via keyboard

### Event Handlers

| Event | Handler | Purpose | Requirement |
|-------|---------|---------|-------------|
| `click` | `handleTabClick(event)` | Switch to clicked tab, save to localStorage | FR-001, FR-009 |
| `keydown` | `handleKeyDown(event)` | Handle Enter, Space, Arrow Left/Right | FR-014, FR-015 |
| `focus` | (automatic CSS) | Show focus indicator | FR-016 |

### Relationships

- **Controls**: Exactly 1 TabContent (via `aria-controls` attribute)
- **Part of**: Exactly 1 Tab List (Settings, Transactions, or Mining page)
- **State sync**: Updates TabState in localStorage on activation

---

## 4. Entity: TabContent

### Description
Content section displayed when corresponding tab is active. Hidden when tab is inactive.

### DOM Structure

```html
<div
    id="tab-panel-node"
    class="tab-content active"
    role="tabpanel"
    aria-labelledby="tab-btn-node"
    tabindex="0"
>
    <!-- Node configuration form content -->
</div>
```

### Properties (DOM Attributes)

| Property | Type | Required | Description | Requirement |
|----------|------|----------|-------------|-------------|
| `id` | string | Yes | Content panel ID (`tab-panel-{tabId}` or `tab-{tabId}`) | FR-005, FR-006 |
| `class` | string | Yes | CSS class (`.tab-content`, `.tab-content.active` when visible) | FR-005, FR-006 |
| `role` | string | Yes | ARIA role `"tabpanel"` | NFR-007 |
| `aria-labelledby` | string | Yes | ID of controlling tab button (`tab-btn-{tabId}`) | NFR-007 |
| `tabindex` | number | Optional | `0` to make panel focusable (for screen readers) | NFR-007 |

### Visibility States

| State | CSS Class | Display | Requirement |
|-------|-----------|---------|-------------|
| **Active** | `.tab-content.active` | `display: block` | FR-005, FR-008 |
| **Inactive** | `.tab-content` | `display: none` | FR-006 |

### Content Types (Per Page)

**Settings Page Panels**:
- `tab-panel-network`: Network configuration form (network type, ports)
- `tab-panel-node`: Node management controls (start/stop node)
- `tab-panel-application`: Application settings (theme, language, etc.)
- `tab-panel-security`: Security settings (password, encryption)

**Transactions Page Panels**:
- `tab-panel-receive`: Receive address display + QR code
- `tab-panel-history`: Transaction history list/table
- `tab-panel-addressbook`: Saved addresses list

**Mining Page Panels**:
- `tab-panel-configure`: Mining configuration options (threads, intensity)
- `tab-panel-history`: Mining activity history/logs

### Behavior Rules

**FR-005**: System MUST show only the content section corresponding to active tab
**FR-006**: System MUST hide all non-active tab content sections
**FR-007**: System MUST transition between tabs without page reload
**FR-008**: System MUST render content immediately when tab becomes active (no loading delay for static content)

### Performance Characteristics

**NFR-005**: Tab switching MUST not cause UI freezes or jank (60fps maintained)
- Content switching uses CSS display property (fast, no reflow)
- No JavaScript content loading (content pre-rendered in HTML)
- No animations (instant switch)

**NFR-006**: localStorage operations MUST not introduce perceptible delay
- localStorage read/write < 1ms typical
- Does not block content rendering

### Relationships

- **Controlled by**: Exactly 1 TabButton (via `aria-labelledby` attribute)
- **Part of**: Exactly 1 Page (Settings, Transactions, or Mining)
- **Contains**: Form fields, lists, or other UI elements (varies by tab)

---

## 5. TabManager Class (Implementation Entity)

### Description
JavaScript class managing tab switching logic for a single page. Not a data entity, but included here as the **coordination layer** between TabState, TabButtons, and TabContents.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `page` | string | Page identifier ('settings', 'transactions', 'mining') |
| `defaultTab` | string | Default tab ID (first tab, fallback when no localStorage) |
| `onTabChange` | function | Optional callback when tab changes |

### Methods

| Method | Parameters | Returns | Purpose |
|--------|------------|---------|---------|
| `init()` | none | void | Load saved tab, set up event listeners |
| `handleTabClick(event)` | Event | void | Handle tab button click |
| `handleKeyDown(event)` | Event | void | Handle keyboard navigation |
| `activateTab(tabId, save)` | string, boolean | void | Activate specified tab, optionally save to localStorage |
| `saveActiveTab(tabId)` | string | void | Save tab ID to localStorage (updates TabState) |
| `loadActiveTab()` | none | string | Load tab ID from localStorage (reads TabState) |
| `navigateTabs(key, currentBtn)` | string, Element | void | Arrow key navigation between tabs |

### Lifecycle

```
Page Load
  ↓
new TabManager({ page: 'settings', defaultTab: 'node' })
  ↓
init()
  ├── loadActiveTab() → Reads TabState from localStorage
  ├── activateTab(savedTabId, false) → Updates DOM (TabButton + TabContent)
  └── Set up event delegation on .tab-nav
  ↓
User Interaction (click or keyboard)
  ↓
handleTabClick() or handleKeyDown()
  ↓
activateTab(newTabId, true)
  ├── Update TabButton states (CSS classes, ARIA attributes)
  ├── Update TabContent visibility (CSS classes)
  └── saveActiveTab(newTabId) → Writes TabState to localStorage
  ↓
Optional: onTabChange callback
```

---

## 6. Data Flow Diagram

```
User Action (Click Tab Button)
  ↓
TabButton.click event
  ↓
TabManager.handleTabClick(event)
  ↓
TabManager.activateTab(tabId, save=true)
  ↓
┌─────────────────────────────────────────┐
│ 1. Update All TabButtons                │
│    - Remove .active class               │
│    - Set aria-selected="false"          │
│    - Set tabindex="-1"                  │
└─────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────┐
│ 2. Update Target TabButton              │
│    - Add .active class                  │
│    - Set aria-selected="true"           │
│    - Set tabindex="0"                   │
└─────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────┐
│ 3. Update All TabContents               │
│    - Remove .active class               │
│    - (display: none via CSS)            │
└─────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────┐
│ 4. Update Target TabContent             │
│    - Add .active class                  │
│    - (display: block via CSS)           │
└─────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────┐
│ 5. Persist TabState to localStorage     │
│    localStorage.setItem(                │
│      'btpc_active_tab_{page}', tabId    │
│    )                                    │
└─────────────────────────────────────────┘
  ↓
Visual Update Complete (< 50ms, NFR-001)
```

---

## 7. Validation Rules

### TabState Validation

| Rule | Check | Action on Failure |
|------|-------|-------------------|
| Page must be valid | `['settings', 'transactions', 'mining'].includes(page)` | Throw error (developer mistake) |
| TabId must exist for page | `validTabIds[page].includes(tabId)` | Use defaultTabId, clear localStorage |
| localStorage available | Try/catch on `localStorage.setItem()` | Log warning, continue (graceful degradation) |

### TabButton Validation

| Rule | Check | Requirement |
|------|-------|-------------|
| Must have data-tab attribute | `tabBtn.dataset.tab !== undefined` | FR-001 |
| Must have role="tab" | `tabBtn.getAttribute('role') === 'tab'` | NFR-007 |
| Must have aria-selected | `tabBtn.hasAttribute('aria-selected')` | NFR-007 |
| Must have aria-controls | `tabBtn.hasAttribute('aria-controls')` | NFR-007 |

### TabContent Validation

| Rule | Check | Requirement |
|------|-------|-------------|
| Must exist for button | `document.getElementById('tab-panel-{tabId}') !== null` | FR-005 |
| Must have role="tabpanel" | `panel.getAttribute('role') === 'tabpanel'` | NFR-007 |
| Must have aria-labelledby | `panel.hasAttribute('aria-labelledby')` | NFR-007 |

---

## 8. Performance Characteristics

### Memory Footprint

| Entity | Size | Count | Total |
|--------|------|-------|-------|
| TabState (localStorage) | ~20 bytes per entry | 3 entries (1 per page) | ~60 bytes |
| TabButton (DOM) | ~1 KB per button | ~9 buttons total (across 3 pages) | ~9 KB |
| TabContent (DOM) | ~5-50 KB per panel (varies by content) | ~9 panels total | ~50-500 KB |
| TabManager instance | ~2 KB per instance | 3 instances (1 per page) | ~6 KB |

**Total**: < 1 MB (negligible compared to full desktop app)

### Operation Performance

| Operation | Time | Requirement |
|-----------|------|-------------|
| Load TabState from localStorage | < 1 ms | NFR-006 |
| Save TabState to localStorage | < 1 ms | NFR-006 |
| Update DOM (switch tabs) | < 5 ms (class manipulation) | NFR-001 (< 50ms total) |
| CSS transition rendering | 0 ms (instant, no animations) | NFR-001 |
| Total tab switch time | < 10 ms | NFR-001 (< 50ms required), NFR-005 (60fps) |

---

## 9. Edge Case Handling

### Missing TabContent

**Scenario**: Tab button exists but corresponding panel missing from DOM
**Detection**: `document.getElementById('tab-panel-{tabId}') === null`
**Action**: Log warning, still activate button (no crash)

```javascript
if (targetContent) {
    targetContent.classList.add('active');
} else {
    console.warn(`TabContent #tab-panel-${tabId} not found for page ${this.page}`);
}
```

### localStorage Disabled

**Scenario**: User has disabled localStorage in browser settings
**Detection**: Try/catch on `localStorage.setItem()` throws exception
**Action**: Continue with tab switching, just don't persist (graceful degradation)

```javascript
try {
    localStorage.setItem(`btpc_active_tab_${this.page}`, tabId);
} catch (e) {
    console.warn('localStorage unavailable, tab state will not persist:', e);
}
```

### Rapid Clicks

**Scenario**: User clicks multiple tabs in quick succession
**Behavior**: Last click wins (no race condition, DOM updates are synchronous)
**Performance**: No debouncing needed (< 10ms per operation)

---

## 10. Article XI Compliance (Constitutional)

### Single Source of Truth (Section 11.1)

**TabState Storage**: localStorage (browser-native, not backend)
- Compliant: Tab selection is **non-critical UI preference**, not authoritative blockchain/wallet data
- Frontend displays tab, does not maintain critical state

### Backend-First Validation (Section 11.2)

**Not Applicable**: Tab switching has no backend validation
- No data modification (pure UI navigation)
- No blockchain/wallet state changes
- localStorage save occurs immediately (no backend roundtrip)

### Event-Driven Architecture (Section 11.3)

**Not Applicable**: Tab state is page-local
- No cross-page synchronization needed
- No backend events emitted for tab changes

### Event Listener Cleanup (Section 11.6)

**Compliant**: Event listeners attached to DOM elements
- TabManager listeners on `.tab-nav` (DOM element)
- When page unloads, DOM destroyed → listeners garbage collected
- No global persistent listeners

---

**Data Model Complete**: All entities defined, ready for task generation in Phase 2
