# Quickstart Test Plan: Fix Non-Functional Sub-Tabs

**Feature**: 004-fix-non-functional
**Date**: 2025-10-25
**Purpose**: Manual test plan to validate tab switching functionality against acceptance criteria

---

## Prerequisites

**Before Testing**:
1. ✅ BTPC desktop app installed and functional
2. ✅ btpc-tab-manager.js module created and included in HTML files
3. ✅ Settings, Transactions, and Mining HTML files updated with tab functionality
4. ✅ CSS enhancements applied (focus states, active states)
5. ✅ Browser DevTools available (for checking localStorage and console errors)

**Environment**:
- Desktop: Tauri 2.0 desktop app
- Browser Engine: Chromium (embedded in Tauri webview)
- localStorage: Enabled (check Settings → Privacy)

---

## Test Execution Checklist

Run tests in order. Mark ✅ PASS or ❌ FAIL for each scenario.

---

## Part 1: Settings Page Navigation (Scenarios 1-3)

### Scenario 1: Click Node Tab
**Given**: User is on Settings page
**When**: User clicks "Node" tab button
**Then**:
- [ ] Node configuration content displays
- [ ] "Node" tab button shows active state (gold border-bottom, bold text)
- [ ] No JavaScript errors in console (F12 → Console tab)

**Steps**:
1. Launch BTPC desktop app
2. Navigate to Settings page (sidebar menu)
3. Click "Node" tab button
4. Observe content area changes to Node configuration
5. Observe tab button has gold underline and bolder text
6. Open DevTools (F12), check Console for errors

**Expected localStorage**:
```javascript
localStorage.getItem('btpc_active_tab_settings') === 'node'
```

---

### Scenario 2: Click Application Tab from Node Tab
**Given**: User is on Settings page with "Node" tab active
**When**: User clicks "Application" tab button
**Then**:
- [ ] Node content hides
- [ ] Application content displays
- [ ] "Application" tab shows active state
- [ ] "Node" tab shows inactive state (gray text, no underline)

**Steps**:
1. From Scenario 1 (Node tab active)
2. Click "Application" tab button
3. Observe Node content disappears immediately
4. Observe Application content appears immediately
5. Observe "Application" tab has active styling
6. Observe "Node" tab has inactive styling

**Performance Validation**:
- Tab switch should feel instant (< 50ms, no lag)
- No visual jank or stuttering

---

### Scenario 3: Click Security Tab
**Given**: User is on Settings page with "Application" tab active
**When**: User clicks "Security" tab button
**Then**:
- [ ] Security content displays as active tab

**Steps**:
1. From Scenario 2 (Application tab active)
2. Click "Security" tab button
3. Observe Security content displays
4. Observe "Security" tab has active styling

---

## Part 2: Transactions Page Navigation (Scenarios 4-6)

### Scenario 4: Click Receive Tab
**Given**: User is on Transactions page
**When**: User clicks "Receive" tab button
**Then**:
- [ ] Receive address/QR code content displays
- [ ] "Receive" tab button shows active state

**Steps**:
1. Navigate to Transactions page (sidebar menu)
2. Click "Receive" tab button
3. Observe Receive address and QR code display
4. Observe "Receive" tab has active styling

**Expected localStorage**:
```javascript
localStorage.getItem('btpc_active_tab_transactions') === 'receive'
```

---

### Scenario 5: Click History Tab
**Given**: User is on Transactions page with "Receive" tab active
**When**: User clicks "History" tab button
**Then**:
- [ ] Transaction history list displays
- [ ] "History" tab shows active state
- [ ] "Receive" tab shows inactive state

**Steps**:
1. From Scenario 4 (Receive tab active)
2. Click "History" tab button
3. Observe transaction history list/table displays
4. Observe "History" tab has active styling

---

### Scenario 6: Click Address Book Tab
**Given**: User is on Transactions page
**When**: User clicks "Address Book" tab button
**Then**:
- [ ] Saved addresses content displays
- [ ] "Address Book" tab shows active state

**Steps**:
1. From Scenario 5
2. Click "Address Book" tab button
3. Observe saved addresses list displays
4. Observe "Address Book" tab has active styling

---

## Part 3: Mining Page Navigation (Scenarios 7-8)

### Scenario 7: Click Configure Tab
**Given**: User is on Mining page
**When**: User clicks "Configure" tab button
**Then**:
- [ ] Mining configuration options display
- [ ] "Configure" tab button shows active state

**Steps**:
1. Navigate to Mining page (sidebar menu)
2. Click "Configure" tab button
3. Observe mining configuration form displays
4. Observe "Configure" tab has active styling

**Expected localStorage**:
```javascript
localStorage.getItem('btpc_active_tab_mining') === 'configure'
```

---

### Scenario 8: Click History Tab
**Given**: User is on Mining page with "Configure" tab active
**When**: User clicks "History" tab button
**Then**:
- [ ] Mining activity history displays
- [ ] "History" tab shows active state
- [ ] "Configure" tab shows inactive state

**Steps**:
1. From Scenario 7 (Configure tab active)
2. Click "History" tab button
3. Observe mining activity history/logs display
4. Observe "History" tab has active styling

---

## Part 4: State Persistence (Scenarios 9-11)

### Scenario 9: Navigate Away and Return
**Given**: User has selected "Security" tab on Settings page
**When**: User navigates to another page and returns to Settings
**Then**:
- [ ] "Security" tab remains active (not reset to first tab)

**Steps**:
1. Go to Settings page
2. Click "Security" tab (should be active)
3. Navigate to Transactions page (sidebar menu)
4. Navigate back to Settings page (sidebar menu)
5. Observe "Security" tab is still active (not "Node" or "Network")

**Verify localStorage**:
```javascript
localStorage.getItem('btpc_active_tab_settings') === 'security'
```

---

### Scenario 10: Refresh Browser
**Given**: User has selected "History" tab on Transactions page
**When**: User refreshes the browser
**Then**:
- [ ] "History" tab remains active after refresh

**Steps**:
1. Go to Transactions page
2. Click "History" tab
3. Refresh page (F5 or Ctrl+R / Cmd+R)
4. Observe "History" tab is active after page reload

**Verify localStorage persists**:
```javascript
// Before refresh
localStorage.getItem('btpc_active_tab_transactions') === 'history'

// After refresh (should be same)
localStorage.getItem('btpc_active_tab_transactions') === 'history'
```

---

### Scenario 11: First-Time User (Default Tab)
**Given**: User opens BTPC desktop app for first time
**When**: User navigates to Settings page
**Then**:
- [ ] First tab displays as default active tab

**Steps**:
1. Clear localStorage for Settings page:
   ```javascript
   localStorage.removeItem('btpc_active_tab_settings')
   ```
2. Navigate to Settings page
3. Observe first tab (leftmost, likely "Network" or "Node") is active by default

**Note**: "First tab" is whichever button appears first in HTML (currently "Network" on settings.html)

---

## Part 5: Keyboard Navigation (Scenarios 12-13)

### Scenario 12: Tab Key + Enter
**Given**: User is on Settings page
**When**: User presses Tab key to focus "Application" tab button and presses Enter
**Then**:
- [ ] Application content displays
- [ ] "Application" tab becomes active
- [ ] Visible focus indicator appears (gold outline)

**Steps**:
1. Go to Settings page
2. Press Tab key repeatedly until "Application" tab button is focused
3. Observe gold outline around "Application" button (focus indicator)
4. Press Enter key
5. Observe Application content displays
6. Observe "Application" tab has active styling

---

### Scenario 13: Arrow Key Navigation
**Given**: User is on Mining page with keyboard focus on tab buttons
**When**: User presses Arrow Right key
**Then**:
- [ ] Focus moves to next tab button
- [ ] That tab activates

**Steps**:
1. Go to Mining page
2. Click "Configure" tab button to give it focus
3. Press Arrow Right key
4. Observe focus moves to "History" tab button (gold outline)
5. Observe "History" content displays (tab auto-activates OR press Enter to activate, depending on implementation)

**Additional Keyboard Tests**:
- [ ] Arrow Left navigates to previous tab
- [ ] Arrow Right wraps around to first tab when on last tab
- [ ] Arrow Left wraps around to last tab when on first tab
- [ ] Space key activates focused tab (same as Enter)

---

## Part 6: Edge Cases

### Edge Case 1: localStorage Disabled
**Test**: Disable localStorage in browser

**Steps**:
1. In DevTools Console, delete localStorage mock:
   ```javascript
   // Simulate disabled localStorage (throw on access)
   const ls = localStorage;
   Object.defineProperty(window, 'localStorage', {
       get() { throw new Error('localStorage disabled'); }
   });
   ```
2. Navigate to Settings page
3. Click tabs

**Expected Behavior**:
- [ ] Tabs still switch (content changes, visual feedback works)
- [ ] Console shows warning: "localStorage unavailable"
- [ ] Tab selection NOT persisted (resets on refresh)
- [ ] No crash or error stopping functionality

**Restore localStorage**:
```javascript
Object.defineProperty(window, 'localStorage', { value: ls });
```

---

### Edge Case 2: Rapid Tab Clicking
**Test**: Click multiple tabs very quickly

**Steps**:
1. Go to Settings page
2. Rapidly click: Node → Application → Security → Node → Security (as fast as possible)
3. Observe final state

**Expected Behavior**:
- [ ] Only final clicked tab ("Security") is active
- [ ] No visual glitches (multiple tabs active, flickering, etc.)
- [ ] Content matches active tab (Security content visible)
- [ ] No JavaScript errors in console

---

### Edge Case 3: Missing Tab Content
**Test**: Tab button exists but content div missing

**Setup**:
1. In DevTools Elements panel, temporarily delete a tab content div (e.g., `#tab-panel-security`)
2. Click "Security" tab button

**Expected Behavior**:
- [ ] Tab button still activates (gold underline)
- [ ] No crash or error
- [ ] Console shows warning: "TabContent #tab-panel-security not found"
- [ ] Other tabs still functional

**Restore**: Undo element deletion in DevTools

---

## Part 7: Accessibility Verification

### ARIA Attributes Validation

**Steps**:
1. Open DevTools → Elements tab
2. Inspect a tab button element
3. Verify ARIA attributes present:
   ```html
   <button
       role="tab"
       aria-selected="true"  <!-- or "false" if inactive -->
       aria-controls="tab-panel-node"
       tabindex="0"  <!-- or "-1" if inactive -->
   >
   ```

**Checklist**:
- [ ] Tab buttons have `role="tab"`
- [ ] Active tab has `aria-selected="true"`, others have `"false"`
- [ ] Active tab has `tabindex="0"`, others have `"-1"`
- [ ] Each button has `aria-controls` pointing to panel ID
- [ ] Tab container has `role="tablist"`
- [ ] Tab panels have `role="tabpanel"`
- [ ] Tab panels have `aria-labelledby` pointing to button ID

---

### Screen Reader Test (Optional)

**Requires**: Screen reader software (NVDA on Windows, VoiceOver on macOS)

**Steps**:
1. Enable screen reader
2. Navigate to Settings page
3. Use Tab key to focus tab buttons
4. Listen to screen reader announcements

**Expected Announcements** (example):
- "Node, tab, 1 of 4, selected" (active tab)
- "Application, tab, 2 of 4" (inactive tab)
- "Security, tab, 3 of 4" (inactive tab)

**Checklist**:
- [ ] Screen reader announces tab role
- [ ] Screen reader announces selected state
- [ ] Screen reader announces tab position (1 of 4)
- [ ] Screen reader announces tab label

---

### Color Contrast Validation

**Tool**: DevTools Accessibility Inspector or Contrast Checker

**Steps**:
1. Right-click "Node" tab button (active)
2. Inspect → Accessibility panel
3. Check color contrast ratio

**Requirements** (WCAG 2.1 AA):
- [ ] Active tab text has ≥ 4.5:1 contrast ratio (gold `#d4af37` on dark background)
- [ ] Inactive tab text has ≥ 4.5:1 contrast ratio
- [ ] Focus outline has ≥ 3:1 contrast ratio

---

## Part 8: Performance Validation

### Tab Switch Response Time

**Tool**: DevTools Performance profiler

**Steps**:
1. Open DevTools → Performance tab
2. Click "Record" (red circle)
3. Click a tab button (e.g., "Application")
4. Stop recording
5. Analyze timeline

**Requirements**:
- [ ] Visual response (class change) occurs in < 50ms (NFR-001)
- [ ] No long tasks (> 50ms yellow bars in timeline)
- [ ] Frame rate stays at 60fps (no red dropped frames)

---

### localStorage Performance

**Test**: Measure localStorage read/write time

**Steps**:
1. Open DevTools Console
2. Run performance test:
   ```javascript
   console.time('localStorage write');
   localStorage.setItem('btpc_active_tab_settings', 'security');
   console.timeEnd('localStorage write');

   console.time('localStorage read');
   localStorage.getItem('btpc_active_tab_settings');
   console.timeEnd('localStorage read');
   ```

**Requirements**:
- [ ] localStorage write < 5ms (NFR-006)
- [ ] localStorage read < 5ms (NFR-006)
- [ ] No perceptible delay during tab switching

---

## Test Summary

### Pass Criteria

**All tests must pass**:
- ✅ 8 navigation scenarios (1-8): All tabs switch correctly
- ✅ 3 state persistence scenarios (9-11): localStorage works, defaults correct
- ✅ 2 keyboard navigation scenarios (12-13): Arrow keys, Enter, Tab work
- ✅ 3 edge cases: localStorage disabled, rapid clicks, missing content handled gracefully
- ✅ Accessibility: ARIA attributes present, screen reader compatible
- ✅ Performance: < 50ms response, 60fps, no localStorage delay

**Failure Criteria** (any of these fails the test):
- ❌ Tab click does nothing (content doesn't change)
- ❌ Multiple tabs active simultaneously
- ❌ JavaScript errors in console during normal operation
- ❌ Tab state not persisted (resets on page navigation or refresh)
- ❌ Keyboard navigation doesn't work
- ❌ ARIA attributes missing or incorrect
- ❌ Color contrast below WCAG 2.1 AA (< 4.5:1)
- ❌ Tab switching causes lag or jank (> 50ms, dropped frames)

---

## Regression Testing

### After Any Code Changes

Re-run critical paths:
1. Scenario 1 (Settings Node tab click)
2. Scenario 10 (Refresh browser, tab persists)
3. Scenario 12 (Keyboard Tab + Enter)
4. Edge Case 2 (Rapid clicking)

If all pass, full feature likely still functional.

---

## Bug Reporting Template

If a test fails, report using this template:

```
**Test Failed**: Scenario X - [Name]
**Page**: Settings / Transactions / Mining
**Steps to Reproduce**:
1. [Step 1]
2. [Step 2]
3. [Step 3]

**Expected**: [What should happen]
**Actual**: [What actually happened]
**Console Errors**: [Copy/paste any errors from DevTools Console]
**localStorage State**: [Run `localStorage.getItem('btpc_active_tab_settings')` and paste result]
**Screenshot**: [Attach if visual bug]
```

---

## Completion Checklist

After running all tests:

- [ ] All 13 acceptance scenarios pass
- [ ] All 3 edge cases handled correctly
- [ ] ARIA attributes validated
- [ ] Performance requirements met (< 50ms, 60fps)
- [ ] No JavaScript console errors
- [ ] localStorage persistence verified
- [ ] Keyboard navigation functional
- [ ] Regression tests pass

**Sign-Off**: Feature ready for production deployment when all boxes checked.

---

**Quickstart Test Plan Complete**: Ready for manual validation in Phase 5
