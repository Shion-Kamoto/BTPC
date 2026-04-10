# BTPC Professional Icon SVG Sources

This directory contains the editable source SVG files for all BTPC professional icons.

## Files

- `icon-home-pro.svg` - Dashboard/Home navigation
- `icon-wallet-pro.svg` - Wallet/Portfolio management
- `icon-transactions-pro.svg` - Transaction history and activity
- `icon-mining-pro.svg` - Mining operations and pool management
- `icon-node-pro.svg` - Network node and peer connections
- `icon-settings-pro.svg` - Settings and configuration
- `icon-send-pro.svg` - Send/Transfer funds
- `icon-receive-pro.svg` - Receive funds and generate addresses
- `icon-address-pro.svg` - Address display and QR codes
- `icon-balance-pro.svg` - Balance and fund display
- `icon-status-pro.svg` - Status indicators and information
- `icon-security-pro.svg` - Security and encryption features

## Design Specifications

### Technical Parameters
- **ViewBox**: 24×24 (always)
- **Stroke Width**: 2.5px (standard), 3px (emphasis)
- **Stroke Linecap**: Round
- **Stroke Linejoin**: Round
- **Fill Strategy**: Use `fill-opacity` for depth (15-80%)
- **Color**: `stroke="currentColor"` for theme support

### Design Principles

1. **Bold & Clear**
   - Minimum 2.5px stroke weight
   - Strategic fills for contrast
   - No fine details that disappear at small sizes

2. **High Contrast**
   - Filled shapes combined with strokes
   - Multiple opacity levels for depth
   - Clear silhouettes at all sizes

3. **Professional**
   - Financial application appropriate
   - Cryptocurrency-specific metaphors
   - Consistent design language

4. **Functional**
   - Optimized for 16-20px display
   - Instant recognition
   - Clear at small sizes

## Editing Guidelines

### Tools
- **Vector Editors**: Figma, Sketch, Illustrator, Inkscape
- **SVG Optimization**: SVGO (optional, preserve structure)
- **Preview**: Test at 16px, 20px, 24px sizes

### Workflow

1. **Open SVG file** in your editor
2. **Maintain viewBox** at 24×24
3. **Keep stroke weight** at 2.5-3px
4. **Test visibility** at 16px minimum
5. **Verify in both** light and dark themes
6. **Save as optimized SVG**

### Adding Fills

Good fill usage:
```svg
<!-- Body with subtle fill -->
<rect x="3" y="6" width="18" height="13" rx="2" 
      fill="currentColor" fill-opacity="0.15"/>

<!-- Emphasis with strong fill -->
<circle cx="17" cy="14" r="2" 
        fill="currentColor"/>
```

### Stroke Best Practices

```svg
<!-- Standard elements -->
<path d="M..." stroke="currentColor" stroke-width="2.5" 
      stroke-linecap="round" stroke-linejoin="round"/>

<!-- Emphasis elements -->
<path d="M..." stroke="currentColor" stroke-width="3" 
      stroke-linecap="round" stroke-linejoin="round"/>
```

## Converting to Data URI

After editing, convert to CSS data URI:

### Manual Method
1. Minify SVG (remove whitespace)
2. URL-encode: Replace spaces, quotes, etc.
3. Prefix with `data:image/svg+xml,`

### Automated Method
```bash
# Using Node.js
node -e "console.log('data:image/svg+xml,' + encodeURIComponent(require('fs').readFileSync('icon.svg', 'utf8')))"

# Using Python
python3 -c "import urllib.parse; print('data:image/svg+xml,' + urllib.parse.quote(open('icon.svg').read()))"
```

### Update CSS
Add to `icons-professional.css`:
```css
.icon-[name]-pro {
    background-image: url("data:image/svg+xml,[ENCODED_DATA]");
    background-size: contain;
    background-repeat: no-repeat;
    background-position: center;
}
```

## Testing Checklist

Before finalizing changes:

- [ ] Test at 16px size (minimum)
- [ ] Test at 20px size (standard)
- [ ] Test at 24px size (comfortable)
- [ ] Test on dark background (#1a1a2e)
- [ ] Test on light background (#ffffff)
- [ ] Verify stroke weight consistency
- [ ] Check fill opacity visibility
- [ ] Ensure clear silhouette
- [ ] Compare with other icons for consistency
- [ ] Validate SVG syntax

## Common Issues

### Icon too thin at small sizes
**Solution**: Increase stroke-width to 3px or add strategic fills

### Loss of detail at 16px
**Solution**: Simplify shapes, remove fine details, increase contrast

### Poor visibility on dark background
**Solution**: Add filled elements with 15-30% opacity

### Inconsistent with other icons
**Solution**: Match stroke weight, cap/join style, and fill strategy

## Version Control

When updating icons:

1. Document changes in commit message
2. Update version in main CSS file
3. Regenerate demo page if needed
4. Update documentation if design principles change

## Icon Design Templates

### Basic Structure
```svg
<svg xmlns="http://www.w3.org/2000/svg" 
     viewBox="0 0 24 24" 
     fill="none" 
     stroke="currentColor" 
     stroke-width="2.5" 
     stroke-linecap="round" 
     stroke-linejoin="round">
  
  <!-- Fill layers (background to foreground) -->
  <rect fill="currentColor" fill-opacity="0.15" .../>
  
  <!-- Stroke outlines -->
  <path d="..." />
  
  <!-- Emphasis elements -->
  <circle fill="currentColor" .../>
</svg>
```

### Layer Order (back to front)
1. Large fills (10-20% opacity)
2. Medium fills (20-40% opacity)
3. Stroke outlines (2.5px)
4. Emphasis fills (60-100% opacity)
5. Bold strokes (3px)

## Resources

- **Design Guide**: `../ICON_DESIGN_GUIDE.md`
- **Quick Reference**: `../ICON_QUICK_REFERENCE.md`
- **Demo Page**: `../icons-demo.html`
- **Production CSS**: `../icons-professional.css`

## Support

For questions or issues:
1. Review design guide
2. Check demo page for examples
3. Compare with existing icons
4. Test at all required sizes
5. Validate accessibility (contrast)
