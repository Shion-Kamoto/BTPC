# UI Healer Command

## Overview
Analyzes the current BTPC application UI against the established style guide and UX rules, then provides specific, actionable recommendations to align the interface with design standards.

## Usage
```
/ui-healer [url]
```

## Parameters
- `url` (optional): Specific page URL to analyze. If not provided, analyzes the current page or prompts for URL.

## What This Command Does

### Step 1: Screenshot Capture
Take a screenshot of each screen in question using the Playwright MCP to capture current UI state.

### Step 2: Style Guide Analysis
Reference the `/style-guide/` directory and find the files `style-guide.md`, `BTPC-GUI-guide.md` , `BTPC/.playwright-mcp` and `ux-rules.md`. Based on those files, grade the outputs of step 1 objectively against that standard, and give a response on a scale of 1 to 10.

### Step 3: Iterative Improvement
For any screenshot or components that have a score of less than 9 out of 10, make changes to the UI code and apply the changes to /home/bob/BTPC/btpc-desktop-app, and then repeat from step 1 until all components score 8+ out of 10.

## Detailed Analysis Criteria

### Layout Architecture (1-10 points)
- **280px left sidebar navigation**: Required layout structure
- **Main content area**: Dynamic right panel (80% width)
- **Split ratio compliance**: 20% sidebar, 80% content
- **Responsive breakpoints**: Minimum 1024px window width

### Color Scheme Implementation (1-10 points)
- **BTPC Blue**: `#1a365d` (Primary brand color usage)
- **BTPC Gold**: `#ffd700` (Accent color for highlights)
- **Terminal Colors**: Green (#48bb78), Amber (#ed8936), Red (#f56565)
- **Dark Theme**: Background Dark (#1a202c), Surface Dark (#2d3748)

### Typography Compliance (1-10 points)
- **Fira Code**: Primary monospace font implementation
- **Text hierarchy**: Proper heading sizes (32px, 24px, 20px)
- **Financial data**: Monospace font for addresses/amounts
- **8 decimal places**: BTPC amount display standard

### Component Standards (1-10 points)
- **Button styling**: 8px border radius, proper color usage
- **Card layouts**: 20px padding, Surface Dark background
- **Status indicators**: Color-coded dots with animation
- **Input fields**: 40px height, 4px border radius

### Data Display Rules (1-10 points)
- **Address format**: First 8 + last 8 characters with ellipsis
- **Balance display**: Always show 8 decimal places for BTPC
- **Timestamps**: Relative time with full timestamp on hover
- **Copy functionality**: Hash/address copy buttons

### Accessibility Requirements (1-10 points)
- **Contrast ratios**: Minimum 4.5:1 for normal text
- **Focus indicators**: BTPC Blue outline (2px)
- **Keyboard navigation**: Tab order and shortcuts
- **ARIA labels**: Proper semantic HTML and screen reader support

## Output Format

```
## UI Health Report for [Page Name]

### Overall Score: X/10

### Critical Issues (Must Fix):
- [List of issues scoring below 6/10]

### Major Issues (Should Fix):
- [List of issues scoring 6-7/10]

### Minor Issues (Could Improve):
- [List of issues scoring 8-9/10]

### Section Scores:
- Layout Architecture: X/10
- Color Scheme: X/10
- Typography: X/10
- Component Standards: X/10
- Data Display: X/10
- Accessibility: X/10

### Priority Action Items:
1. [High Priority] - Implementation guidance
2. [Medium Priority] - Implementation guidance
3. [Low Priority] - Implementation guidance

### Next Steps:
[If any scores < 8] Implement fixes and re-run /ui-healer
[If all scores >= 8] UI meets design standards ✅
```

## Implementation Flow
1. Capture screenshots using Playwright MCP
2. Load and parse style guide requirements
3. Analyze each UI section against criteria
4. Generate numerical scores and detailed feedback
5. If scores < 8, provide specific code fixes
6. Re-test after implementation until compliance achieved
7. Document final compliance status

## Integration Points
- Uses `/style-guide/style-guide.md`, `/style-guide/BTPC-GUI-guide.md` for design specs
- Uses `/style-guide/ux-rules.md` for UX requirements
- Integrates with Playwright for automated testing
- Provides CSS/HTML code recommendations
- Tracks improvement iterations automatically

