# Tasks: Fix Non-Functional Sub-Tabs

**Feature**: 004-fix-non-functional
**Input**: Design documents from `/specs/004-fix-non-functional/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md
**Constitution**: Article XI patterns for desktop features

---

## Execution Summary

**Problem**: Sub-tabs on Settings, Transactions, and Mining pages are non-functional due to JavaScript error (`event` is not defined in `switchTab` function).

**Solution**: Create shared `btpc-tab-manager.js` module with event delegation, ARIA-compliant keyboard navigation, and localStorage persistence. Update 3 HTML files to use the new module.

**Complexity**: LOW (pure frontend JavaScript, no backend changes, ~5 hours estimated)

**Article XI Compliance**: All checks passed (localStorage for non-critical UI preference only)

---

## Task Ordering Strategy

1. **Setup**: Optional CSS enhancements (can be skipped if existing styles sufficient)
2. **Core Module**: Create btpc-tab-manager.js (foundation for all pages)
3. **HTML Updates**: Modify settings.html, transactions.html, mining.html [P] (parallel, different files)
4. **Testing**: Manual validation against 13 acceptance scenarios + edge cases

---

## Phase 3.1: Setup & CSS Enhancements (Optional)

### T001 [P] Add focus state CSS to btpc-styles.css
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/btpc-styles.css`
**Purpose**: Add keyboard focus indicator for tab buttons (FR-016, NFR-007)
**Dependencies**: None (optional enhancement)

**Add the following CSS** (if not already present):
```css
/* Keyboard focus indicator (FR-016, NFR-007) */
.tab-btn:focus {
    outline: 2px solid var(--btpc-primary);
    outline-offset: 2px;
}

.tab-btn:focus:not(:focus-visible) {
    outline: none; /* Remove outline for mouse users */
}

/* Enhanced active state for better visibility (NFR-002) */
.tab-btn.active {
    font-weight: 600; /* Slightly bolder for clarity */
}
```

**Validation**:
- [ ] Verify outline appears when tab button focused via Tab key
- [ ] Verify outline does NOT appear when tab button clicked with mouse (focus-visible)
- [ ] Run browser DevTools Accessibility audit (should pass WCAG 2.1 AA)

---

## Phase 3.2: Core Module Implementation

### T002 Create btpc-tab-manager.js module
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/btpc-tab-manager.js`
**Purpose**: Shared tab switching logic module with ARIA support and localStorage persistence
**Dependencies**: T001 (optional, can proceed without it)
**Constitutional Compliance**: Article XI.6 (event listener cleanup via DOM removal)

**Implementation Requirements** (from research.md, section 4):

1. **TabManager Class**:
   - Constructor parameters: `{ page, defaultTab, onTabChange }`
   - Properties: `page`, `defaultTab`, `onTabChange`
   - Methods: `init()`, `handleTabClick(event)`, `handleKeyDown(event)`, `activateTab(tabId, save)`, `saveActiveTab(tabId)`, `loadActiveTab()`, `navigateTabs(key, currentBtn)`, `extractTabId(button)`

2. **Event Delegation** (FR-001):
   - Attach click handler to `.tab-nav` container (not individual buttons)
   - Use `event.target.closest('.tab-btn')` to find clicked button
   - Extract tab ID from button's `data-tab` attribute or onclick attribute

3. **Keyboard Navigation** (FR-013 to FR-016):
   - Handle `Enter` and `Space` keys to activate focused tab
   - Handle `ArrowLeft` and `ArrowRight` keys to navigate between tabs
   - Wrap around when reaching first/last tab

4. **ARIA Attribute Management** (NFR-007):
   - Set `aria-selected="true"` on active tab, `"false"` on others
   - Set `tabindex="0"` on active tab, `"-1"` on others
   - Toggle `.active` CSS class on tab buttons and content panels

5. **localStorage Persistence** (FR-009 to FR-012):
   - Save active tab ID to `localStorage.setItem('btpc_active_tab_{page}', tabId)`
   - Load saved tab on init: `localStorage.getItem('btpc_active_tab_{page}')`
   - Try/catch blocks for localStorage errors (graceful degradation)
   - Default to `defaultTab` if no saved state or localStorage unavailable

6. **Content Visibility** (FR-005, FR-006):
   - Hide all tab content panels (remove `.active` class)
   - Show only active tab content (add `.active` class to `#tab-panel-{tabId}` or `#tab-{tabId}`)

**Code Structure** (from research.md):
```javascript
/**
 * BTPC Tab Manager Module
 * Provides tab switching functionality with localStorage persistence
 * Article XI compliant - no authoritative state in frontend
 */
class TabManager {
    constructor(options) {
        this.page = options.page;
        this.defaultTab = options.defaultTab;
        this.onTabChange = options.onTabChange || null;
        this.init();
    }

    init() {
        const savedTab = this.loadActiveTab();
        this.activateTab(savedTab, false);

        const tabNav = document.querySelector('.tab-nav');
        if (tabNav) {
            tabNav.addEventListener('click', this.handleTabClick.bind(this));
            tabNav.addEventListener('keydown', this.handleKeyDown.bind(this));
        }
    }

    handleTabClick(event) {
        const tabBtn = event.target.closest('.tab-btn');
        if (!tabBtn) return;

        const tabId = tabBtn.dataset.tab || this.extractTabId(tabBtn);
        this.activateTab(tabId, true);
    }

    handleKeyDown(event) {
        if (!['ArrowLeft', 'ArrowRight', 'Enter', ' '].includes(event.key)) return;

        const tabBtn = event.target.closest('.tab-btn');
        if (!tabBtn) return;

        event.preventDefault();

        if (event.key === 'Enter' || event.key === ' ') {
            const tabId = tabBtn.dataset.tab || this.extractTabId(tabBtn);
            this.activateTab(tabId, true);
        } else {
            this.navigateTabs(event.key, tabBtn);
        }
    }

    activateTab(tabId, save = true) {
        // Remove active from all tabs
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.classList.remove('active');
            btn.setAttribute('aria-selected', 'false');
            btn.setAttribute('tabindex', '-1');
        });

        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.remove('active');
        });

        // Activate target tab
        const targetBtn = document.querySelector(`[data-tab="${tabId}"]`) ||
                         document.querySelector(`.tab-btn[onclick*="${tabId}"]`);
        const targetContent = document.getElementById(`tab-panel-${tabId}`) ||
                             document.getElementById(`tab-${tabId}`);

        if (targetBtn) {
            targetBtn.classList.add('active');
            targetBtn.setAttribute('aria-selected', 'true');
            targetBtn.setAttribute('tabindex', '0');
        }

        if (targetContent) {
            targetContent.classList.add('active');
        } else {
            console.warn(`TabContent #tab-${tabId} not found for page ${this.page}`);
        }

        if (save) {
            this.saveActiveTab(tabId);
        }

        if (this.onTabChange) {
            this.onTabChange(tabId);
        }
    }

    saveActiveTab(tabId) {
        try {
            localStorage.setItem(`btpc_active_tab_${this.page}`, tabId);
        } catch (e) {
            console.warn('localStorage unavailable, tab state will not persist:', e);
        }
    }

    loadActiveTab() {
        try {
            return localStorage.getItem(`btpc_active_tab_${this.page}`) || this.defaultTab;
        } catch (e) {
            console.warn('localStorage unavailable:', e);
            return this.defaultTab;
        }
    }

    navigateTabs(key, currentBtn) {
        const allTabs = Array.from(document.querySelectorAll('.tab-btn'));
        const currentIndex = allTabs.indexOf(currentBtn);

        let nextIndex;
        if (key === 'ArrowRight') {
            nextIndex = (currentIndex + 1) % allTabs.length; // Wrap to first
        } else { // ArrowLeft
            nextIndex = (currentIndex - 1 + allTabs.length) % allTabs.length; // Wrap to last
        }

        const nextBtn = allTabs[nextIndex];
        const nextTabId = nextBtn.dataset.tab || this.extractTabId(nextBtn);

        this.activateTab(nextTabId, true);
        nextBtn.focus();
    }

    extractTabId(button) {
        // Fallback: Extract from onclick attribute (e.g., onclick="switchTab('node')")
        const onclick = button.getAttribute('onclick');
        if (onclick) {
            const match = onclick.match(/switchTab\(['"](.+?)['"]\)/);
            if (match) return match[1];
        }
        return null;
    }
}

// Export for use in pages
window.TabManager = TabManager;
```

**Validation**:
- [ ] File created at `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/btpc-tab-manager.js`
- [ ] TabManager class defined with all methods
- [ ] No syntax errors (run in browser console: `new TabManager({ page: 'test', defaultTab: 'tab1' })`)
- [ ] Event delegation works (no individual onclick handlers needed)
- [ ] localStorage try/catch blocks present (graceful degradation)

---

## Phase 3.3: HTML Updates (Parallel Execution)

### T003 [P] Update settings.html with ARIA attributes and TabManager
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/settings.html`
**Purpose**: Fix non-functional tabs on Settings page (Node/Application/Security/Network)
**Dependencies**: T002 (btpc-tab-manager.js must exist)
**Parallel**: Can run with T004, T005 (different files)

**Changes Required**:

1. **Include btpc-tab-manager.js** (add before closing `</body>`):
   ```html
   <script src="btpc-tab-manager.js"></script>
   ```

2. **Add ARIA attributes to tab buttons**:
   - Add `role="tablist"` to `.tab-nav` container
   - Add `aria-label="Settings Sections"` to `.tab-nav` container
   - For each `<button class="tab-btn">`:
     - Add `role="tab"`
     - Add `id="tab-btn-{tabId}"` (e.g., `id="tab-btn-node"`)
     - Add `data-tab="{tabId}"` (e.g., `data-tab="node"`)
     - Add `aria-selected="false"` (first tab should be `"true"`)
     - Add `aria-controls="tab-panel-{tabId}"` (e.g., `aria-controls="tab-panel-node"`)
     - Add `tabindex="-1"` (first tab should be `"0"`)

3. **Add ARIA attributes to tab content panels**:
   - Change `id="tab-{tabId}"` to `id="tab-panel-{tabId}"` (e.g., `id="tab-panel-node"`)
   - Add `role="tabpanel"`
   - Add `aria-labelledby="tab-btn-{tabId}"` (e.g., `aria-labelledby="tab-btn-node"`)
   - Add `tabindex="0"` (optional, for screen reader focus)

4. **Replace inline switchTab() with TabManager initialization**:
   - Remove the existing `function switchTab(tabName) { ... }` from `<script>` section
   - Remove `onclick="switchTab('...')"` from tab buttons (TabManager uses event delegation)
   - Add initialization in DOMContentLoaded:
     ```html
     <script>
         document.addEventListener('DOMContentLoaded', () => {
             new TabManager({
                 page: 'settings',
                 defaultTab: 'network'  // or 'node' depending on which is leftmost
             });

             // Existing loadSettings() and other code remains below...
             loadSettings();
         });
     </script>
     ```

**Example Before** (current):
```html
<div class="tab-nav">
    <button class="tab-btn active" onclick="switchTab('network')">Network</button>
    <button class="tab-btn" onclick="switchTab('node')">Node</button>
    <button class="tab-btn" onclick="switchTab('application')">Application</button>
    <button class="tab-btn" onclick="switchTab('security')">Security</button>
</div>

<div id="tab-network" class="tab-content active">...</div>
<div id="tab-node" class="tab-content">...</div>

<script>
    function switchTab(tabName) {
        document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));

        event.target.classList.add('active');  // BUG: 'event' not defined
        document.getElementById(`tab-${tabName}`).classList.add('active');
    }
</script>
```

**Example After** (fixed):
```html
<div class="tab-nav" role="tablist" aria-label="Settings Sections">
    <button
        class="tab-btn active"
        role="tab"
        id="tab-btn-network"
        data-tab="network"
        aria-selected="true"
        aria-controls="tab-panel-network"
        tabindex="0"
    >Network</button>
    <button
        class="tab-btn"
        role="tab"
        id="tab-btn-node"
        data-tab="node"
        aria-selected="false"
        aria-controls="tab-panel-node"
        tabindex="-1"
    >Node</button>
    <!-- ... more buttons ... -->
</div>

<div id="tab-panel-network" class="tab-content active" role="tabpanel" aria-labelledby="tab-btn-network" tabindex="0">...</div>
<div id="tab-panel-node" class="tab-content" role="tabpanel" aria-labelledby="tab-btn-node" tabindex="0">...</div>

<script src="btpc-tab-manager.js"></script>
<script>
    document.addEventListener('DOMContentLoaded', () => {
        new TabManager({
            page: 'settings',
            defaultTab: 'network'
        });

        loadSettings(); // Existing function remains
    });
</script>
```

**Validation**:
- [ ] All tab buttons have `role="tab"`, `data-tab`, `aria-selected`, `aria-controls`, `tabindex`
- [ ] All tab content panels have `role="tabpanel"`, `aria-labelledby`, unique IDs
- [ ] TabManager initialized with `page: 'settings'`
- [ ] No inline `onclick` handlers remain on tab buttons
- [ ] Old `switchTab()` function removed
- [ ] Clicking tabs works (no JavaScript errors)
- [ ] localStorage key `btpc_active_tab_settings` created when tab clicked

---

### T004 [P] Update transactions.html with ARIA attributes and TabManager
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/transactions.html`
**Purpose**: Fix non-functional tabs on Transactions page (Receive/History/Address Book)
**Dependencies**: T002 (btpc-tab-manager.js must exist)
**Parallel**: Can run with T003, T005 (different files)

**Changes Required**:

Follow same pattern as T003, but for Transactions page:

1. Include `btpc-tab-manager.js`
2. Add ARIA attributes to tab buttons (Receive, History, Address Book)
3. Add ARIA attributes to tab content panels
4. Initialize TabManager with:
   ```javascript
   new TabManager({
       page: 'transactions',
       defaultTab: 'receive'
   });
   ```

**Tab IDs for Transactions Page** (from research.md):
- `data-tab="receive"` → `id="tab-panel-receive"` (Receive address/QR code)
- `data-tab="history"` → `id="tab-panel-history"` (Transaction history list)
- `data-tab="addressbook"` → `id="tab-panel-addressbook"` (Saved addresses)

**Validation**:
- [ ] All tab buttons have ARIA attributes
- [ ] All tab content panels have ARIA attributes
- [ ] TabManager initialized with `page: 'transactions'`, `defaultTab: 'receive'`
- [ ] Clicking tabs works (no JavaScript errors)
- [ ] localStorage key `btpc_active_tab_transactions` created when tab clicked

---

### T005 [P] Update mining.html with ARIA attributes and TabManager
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/mining.html`
**Purpose**: Fix non-functional tabs on Mining page (Configure/History)
**Dependencies**: T002 (btpc-tab-manager.js must exist)
**Parallel**: Can run with T003, T004 (different files)

**Changes Required**:

Follow same pattern as T003, but for Mining page:

1. Include `btpc-tab-manager.js`
2. Add ARIA attributes to tab buttons (Configure, History)
3. Add ARIA attributes to tab content panels
4. Initialize TabManager with:
   ```javascript
   new TabManager({
       page: 'mining',
       defaultTab: 'configure'
   });
   ```

**Tab IDs for Mining Page** (from research.md):
- `data-tab="configure"` → `id="tab-panel-configure"` (Mining configuration options)
- `data-tab="history"` → `id="tab-panel-history"` (Mining activity history/logs)

**Validation**:
- [ ] All tab buttons have ARIA attributes
- [ ] All tab content panels have ARIA attributes
- [ ] TabManager initialized with `page: 'mining'`, `defaultTab: 'configure'`
- [ ] Clicking tabs works (no JavaScript errors)
- [ ] localStorage key `btpc_active_tab_mining` created when tab clicked

---

## Phase 3.4: Manual Testing (Sequential)

### T006 Execute acceptance scenarios 1-8 (Tab Navigation)
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate tab switching on all 3 pages
**Dependencies**: T003, T004, T005 (all HTML files updated)

**Execute the following scenarios from quickstart.md**:

1. **Settings → Node tab** (Scenario 1)
2. **Settings → Application tab** (Scenario 2)
3. **Settings → Security tab** (Scenario 3)
4. **Transactions → Receive tab** (Scenario 4)
5. **Transactions → History tab** (Scenario 5)
6. **Transactions → Address Book tab** (Scenario 6)
7. **Mining → Configure tab** (Scenario 7)
8. **Mining → History tab** (Scenario 8)

**Pass Criteria** (for each scenario):
- [ ] Clicking tab button changes content immediately (< 50ms)
- [ ] Active tab has gold border-bottom and bold text
- [ ] Inactive tabs have gray text and no border
- [ ] Content area shows only the active tab's content
- [ ] No JavaScript errors in browser console

**Validation**:
- [ ] All 8 scenarios pass
- [ ] Take screenshots of each page with different tabs active (optional)
- [ ] Document any failures with scenario number and observed behavior

---

### T007 Execute state persistence scenarios 9-11
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate localStorage tab state persistence
**Dependencies**: T006 (tab navigation must work)

**Execute the following scenarios from quickstart.md**:

9. **Navigate away and return** (Settings → Security tab, go to Transactions, return to Settings)
   - [ ] Security tab remains active (not reset to first tab)

10. **Refresh browser** (Transactions → History tab, press F5)
    - [ ] History tab remains active after page reload

11. **First-time user** (Clear localStorage, navigate to Settings)
    - [ ] First tab (leftmost) displays as default
    - Run: `localStorage.removeItem('btpc_active_tab_settings')`
    - Navigate to Settings page
    - Verify first tab active

**Validation**:
- [ ] All 3 scenarios pass
- [ ] Verify localStorage keys exist after tab clicks:
  - `localStorage.getItem('btpc_active_tab_settings')`
  - `localStorage.getItem('btpc_active_tab_transactions')`
  - `localStorage.getItem('btpc_active_tab_mining')`

---

### T008 Execute keyboard navigation scenarios 12-13
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate keyboard accessibility (FR-013 to FR-016)
**Dependencies**: T006 (tab navigation must work)

**Execute the following scenarios from quickstart.md**:

12. **Tab key + Enter** (Settings page)
    - [ ] Press Tab key repeatedly until "Application" tab focused
    - [ ] Gold outline appears around button (focus indicator)
    - [ ] Press Enter key
    - [ ] Application content displays, tab becomes active

13. **Arrow key navigation** (Mining page)
    - [ ] Click "Configure" tab to give it focus
    - [ ] Press Arrow Right key
    - [ ] Focus moves to "History" tab (gold outline)
    - [ ] Press Enter to activate OR tab auto-activates (implementation choice)

**Additional Keyboard Tests**:
- [ ] Arrow Left navigates to previous tab
- [ ] Arrow Right wraps around to first tab when on last tab
- [ ] Arrow Left wraps around to last tab when on first tab
- [ ] Space key activates focused tab (same as Enter)

**Validation**:
- [ ] All keyboard navigation works
- [ ] Focus indicator visible (gold outline)
- [ ] No mouse required to switch tabs

---

### T009 Test edge cases (localStorage disabled, rapid clicks, missing content)
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate graceful degradation and robustness
**Dependencies**: T006 (tab navigation must work)

**Execute the following edge cases from quickstart.md**:

**Edge Case 1: localStorage Disabled**
- [ ] In DevTools Console, simulate disabled localStorage (see quickstart.md for code)
- [ ] Navigate to Settings page, click tabs
- [ ] Expected: Tabs still switch, console shows warning, tab selection NOT persisted

**Edge Case 2: Rapid Tab Clicking**
- [ ] Go to Settings page
- [ ] Rapidly click: Node → Application → Security → Node → Security (as fast as possible)
- [ ] Expected: Only final clicked tab ("Security") is active, no visual glitches

**Edge Case 3: Missing Tab Content**
- [ ] In DevTools Elements panel, temporarily delete `#tab-panel-security` div
- [ ] Click "Security" tab button
- [ ] Expected: Button still activates, no crash, console shows warning

**Validation**:
- [ ] All edge cases handled gracefully
- [ ] No crashes or JavaScript errors that stop functionality
- [ ] Console warnings appear as expected

---

### T010 Verify ARIA attributes and screen reader compatibility
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate accessibility compliance (NFR-007, NFR-008)
**Dependencies**: T003, T004, T005 (all HTML files updated with ARIA)

**ARIA Attributes Validation**:

For each page (Settings, Transactions, Mining):
- [ ] Tab container has `role="tablist"`
- [ ] Each tab button has:
  - [ ] `role="tab"`
  - [ ] `aria-selected="true"` (active) or `"false"` (inactive)
  - [ ] `aria-controls` pointing to panel ID
  - [ ] `tabindex="0"` (active) or `"-1"` (inactive)
- [ ] Each tab content panel has:
  - [ ] `role="tabpanel"`
  - [ ] `aria-labelledby` pointing to button ID

**Screen Reader Test** (Optional, requires screen reader software):
- [ ] Enable screen reader (NVDA on Windows, VoiceOver on macOS)
- [ ] Navigate to Settings page
- [ ] Use Tab key to focus tab buttons
- [ ] Listen to announcements (should say "Node, tab, 1 of 4, selected")

**Color Contrast Validation**:
- [ ] Use browser DevTools Accessibility Inspector
- [ ] Check active tab text color vs background (≥ 4.5:1 ratio for WCAG 2.1 AA)
- [ ] Check focus outline color vs background (≥ 3:1 ratio)

**Validation**:
- [ ] All ARIA attributes present and correct
- [ ] Screen reader announces tabs correctly (if tested)
- [ ] Color contrast meets WCAG 2.1 AA

---

### T011 Validate performance (< 50ms response, 60fps)
**File**: `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/quickstart.md`
**Purpose**: Validate performance requirements (NFR-001, NFR-005, NFR-006)
**Dependencies**: T006 (tab navigation must work)

**Tab Switch Response Time** (NFR-001: < 50ms):
- [ ] Open DevTools → Performance tab
- [ ] Click "Record"
- [ ] Click a tab button (e.g., "Application")
- [ ] Stop recording
- [ ] Analyze timeline: Visual response (class change) occurs in < 50ms

**Frame Rate** (NFR-005: 60fps):
- [ ] In Performance timeline, verify no red dropped frames during tab switch
- [ ] Verify no long tasks (> 50ms yellow bars)

**localStorage Performance** (NFR-006: no perceptible delay):
- [ ] In DevTools Console, run:
   ```javascript
   console.time('localStorage write');
   localStorage.setItem('btpc_active_tab_settings', 'security');
   console.timeEnd('localStorage write');
   ```
- [ ] Verify write time < 5ms

**Validation**:
- [ ] Tab switching feels instant (< 50ms)
- [ ] 60fps maintained (no jank)
- [ ] localStorage operations fast (< 5ms)

---

## Phase 3.5: Polish & Documentation

### T012 [P] Update CLAUDE.md with tab switching context
**File**: `/home/bob/BTPC/BTPC/CLAUDE.md`
**Purpose**: Document new btpc-tab-manager.js module for future development
**Dependencies**: T002 (module created)
**Parallel**: Can run with T013, T014 (different files)

**Already Done**: `.specify/scripts/bash/update-agent-context.sh claude` was run during planning phase.

**Verify CLAUDE.md includes**:
- [ ] JavaScript ES6+ in tech stack
- [ ] btpc-tab-manager.js module mentioned
- [ ] localStorage tab state persistence keys

**Optional**: Add brief note about tab switching implementation if not already present.

---

### T013 [P] Run browser DevTools audit
**File**: N/A (manual browser test)
**Purpose**: Verify no JavaScript errors, accessibility issues, or performance problems
**Dependencies**: T006 (all acceptance scenarios pass)
**Parallel**: Can run with T012, T014 (independent)

**Steps**:
1. Open BTPC desktop app
2. Open DevTools (F12)
3. Navigate to Lighthouse tab
4. Run audit for "Accessibility" category
5. Review results

**Expected Results**:
- [ ] No JavaScript errors in Console
- [ ] Accessibility score ≥ 90%
- [ ] ARIA attributes detected and valid
- [ ] Color contrast passes WCAG 2.1 AA

**Validation**:
- [ ] Audit passes
- [ ] Screenshot audit results (optional)
- [ ] Fix any issues found

---

### T014 [P] Final smoke test across all pages
**File**: N/A (manual test)
**Purpose**: Quick regression check that everything still works
**Dependencies**: All implementation tasks complete (T002-T005)
**Parallel**: Can run with T012, T013 (independent)

**Quick Test**:
1. Launch BTPC desktop app
2. Navigate to Settings page
   - [ ] Click each tab (Network, Node, Application, Security)
   - [ ] Verify content changes
3. Navigate to Transactions page
   - [ ] Click each tab (Receive, History, Address Book)
   - [ ] Verify content changes
4. Navigate to Mining page
   - [ ] Click each tab (Configure, History)
   - [ ] Verify content changes
5. Refresh browser (F5)
   - [ ] Navigate to Settings, verify last active tab still active
6. Press Tab key to focus tab button, press Enter
   - [ ] Verify keyboard navigation works

**Validation**:
- [ ] All tabs functional on all pages
- [ ] No regressions from other features
- [ ] Ready for deployment

---

## Dependencies

**Module Dependency**:
- T003, T004, T005 depend on T002 (btpc-tab-manager.js must exist first)

**Testing Dependencies**:
- T006 depends on T003, T004, T005 (all HTML files updated)
- T007 depends on T006 (basic tab navigation must work)
- T008 depends on T006 (basic tab navigation must work)
- T009 depends on T006 (basic tab navigation must work)
- T010 depends on T003, T004, T005 (ARIA attributes added)
- T011 depends on T006 (tab navigation must work)

**Polish Dependencies**:
- T012, T013, T014 depend on T002-T005 (all implementation complete)

---

## Parallel Execution Examples

**Phase 3.3 (HTML Updates) - All Parallel**:
```bash
# Launch T003, T004, T005 together (different files):
# Execute these in separate terminal windows or via Task agents

# Terminal 1:
Task: "Update settings.html with ARIA attributes and TabManager in /home/bob/BTPC/BTPC/btpc-desktop-app/ui/settings.html"

# Terminal 2:
Task: "Update transactions.html with ARIA attributes and TabManager in /home/bob/BTPC/BTPC/btpc-desktop-app/ui/transactions.html"

# Terminal 3:
Task: "Update mining.html with ARIA attributes and TabManager in /home/bob/BTPC/BTPC/btpc-desktop-app/ui/mining.html"
```

**Phase 3.5 (Polish) - All Parallel**:
```bash
# Launch T012, T013, T014 together (independent tasks):

# Terminal 1:
Task: "Update CLAUDE.md with tab switching context"

# Terminal 2:
Task: "Run browser DevTools accessibility audit"

# Terminal 3:
Task: "Final smoke test across all pages"
```

---

## Notes

- **[P] Tasks**: Can run in parallel (different files, no dependencies)
- **Sequential Tasks**: Must run in order (shared dependencies)
- **Article XI Compliance**: Automatically satisfied (event listener cleanup via DOM removal, localStorage for non-critical UI state)
- **No Backend Changes**: Pure frontend JavaScript feature, no Tauri command modifications needed
- **Estimated Total Time**: ~5 hours for implementation + testing

---

## Validation Checklist

**Before Starting**:
- [ ] Read spec.md, research.md, data-model.md, quickstart.md
- [ ] Understand root cause (event not defined in switchTab)
- [ ] Understand solution (shared TabManager module)

**After T002 (Module)**:
- [ ] TabManager class exists with all methods
- [ ] localStorage try/catch blocks present
- [ ] Event delegation implemented
- [ ] Keyboard navigation implemented

**After T003-T005 (HTML Updates)**:
- [ ] All ARIA attributes added
- [ ] Old switchTab() functions removed
- [ ] TabManager initialized on all pages
- [ ] No inline onclick handlers remain

**After T006-T011 (Testing)**:
- [ ] All 13 acceptance scenarios pass
- [ ] State persistence works (navigate away, refresh browser)
- [ ] Keyboard navigation works (Tab, Enter, Arrow keys)
- [ ] Edge cases handled gracefully
- [ ] ARIA attributes validated
- [ ] Performance targets met (< 50ms, 60fps)

**After T012-T014 (Polish)**:
- [ ] CLAUDE.md updated
- [ ] Browser audit passes (accessibility ≥ 90%)
- [ ] Smoke test passes (all tabs functional)

**Ready for Deployment**:
- [ ] All tasks completed
- [ ] All validation checkboxes checked
- [ ] No JavaScript console errors
- [ ] All 3 pages functional

---

## BTPC Project Context

**Feature Type**: Desktop Wallet App (Frontend UI Enhancement)

**Tech Stack**:
- **Language**: JavaScript ES6+ (vanilla, no framework)
- **Dependencies**: btpc-common.js, btpc-storage.js, browser localStorage API
- **Desktop**: Tauri 2.0 webview (Chromium-based)
- **Storage**: Browser localStorage for tab state

**Article XI Compliance**:
- **Section 11.1** (Single Source of Truth): localStorage for UI preferences ✅
- **Section 11.2** (Backend-First Validation): N/A (pure UI navigation) ✅
- **Section 11.3** (Event-Driven Architecture): N/A (local page state) ✅
- **Section 11.6** (Event Listener Cleanup): Automatic via DOM removal ✅
- **Section 11.7** (Prohibited Patterns): No violations ✅

**Performance Targets**:
- Tab switching: < 50ms visual response (NFR-001)
- UI smoothness: 60fps maintained (NFR-005)
- localStorage operations: No perceptible delay (NFR-006)

**Accessibility Standards**:
- WCAG 2.1 AA color contrast (NFR-002)
- ARIA roles: tablist, tab, tabpanel (NFR-007)
- Keyboard navigation: ARIA Authoring Practices Guide (NFR-008)

**Key Documentation**:
- `specs/004-fix-non-functional/spec.md` - Feature specification
- `specs/004-fix-non-functional/research.md` - Research and decisions
- `specs/004-fix-non-functional/data-model.md` - Entity definitions
- `specs/004-fix-non-functional/quickstart.md` - Manual test plan
- `CLAUDE.md` - Project guidelines

---

**Tasks Complete**: Ready for execution via `/implement` or manual task-by-task implementation

**Next Step**: Execute T001 (optional CSS) or skip to T002 (create btpc-tab-manager.js)
