# GPU Mining Rate Limit Fix - 2025-11-16

## Problem Summary
GPU mining was continuously hitting 429 "Rate limit exceeded" errors when requesting block templates from the RPC server, despite having template caching implemented with a 10-second TTL.

## Root Cause Analysis

### Three Critical Bugs Identified:

#### Bug #1: Cache Deletion on Error
**Location**: Lines 408-409
```rust
Err(e) => {
    cached_template = None;  // CLEARS THE CACHE!
    current_template = None;  // CLEARS WORKING TEMPLATE!
```
When a 429 error occurred, the code deleted both the cache AND working template, forcing immediate re-fetch and creating an infinite 429 loop.

#### Bug #2: Incorrect Cache Logic
**Location**: Line 394
```rust
if current_template.is_none() || cached_template.as_ref().map_or(true, |c| !c.is_valid())
```
The logic would fetch a new template even when `current_template` existed if `cached_template` was None (which happens after any error).

#### Bug #3: No Resilience After Errors
After a 429 error:
1. Both templates set to None
2. Sleep 5 seconds
3. Try fetch again → Get another 429
4. Templates set to None again
5. Infinite loop of 429 errors

## Solution Implemented

### Key Changes:
1. **Proper cache validity check** using pattern matching for clarity
2. **Preserve templates on rate limit errors** - continue mining with existing template
3. **Extend cache TTL on rate limit** - prevents repeated fetch attempts
4. **Differentiate error types** - handle rate limits differently from network errors
5. **Longer wait on rate limit** - 30 seconds for rate limit vs 5 seconds for other errors

### Code Changes in `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`:

```rust
// NEW: Clear cache validity check with pattern matching
let should_fetch_template = match (&current_template, &cached_template) {
    (None, _) => true,                                     // No template - must fetch
    (Some(_), Some(cache)) if !cache.is_valid() => true,  // Cache expired
    (Some(_), Some(cache)) if cache.is_valid() => false,  // Cache valid - keep using
    (Some(_), None) => false,                              // Have template, no cache - keep using
};

// NEW: Smart error handling that preserves templates
match rpc_client_clone.get_block_template().await {
    Ok(template) => { /* Update cache and template */ },
    Err(e) => {
        let error_str = e.to_string();
        if error_str.contains("429") || error_str.contains("Too Many Requests") {
            // CRITICAL: Don't clear templates on rate limit!
            if current_template.is_none() {
                // No template - must wait longer
                tokio::time::sleep(Duration::from_secs(30)).await;
            } else {
                // Keep mining with existing template
                // Extend cache TTL to prevent repeated attempts
                if let Some(ref mut cache) = cached_template {
                    cache.cached_at = Instant::now();
                }
            }
        } else {
            // Non-rate-limit error - only clear if no fallback
            if current_template.is_none() {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
```

## Testing Instructions

1. **Rebuild complete** - Run `cargo build --release` ✅ (Already done)

2. **Start GPU mining** and observe logs:
   ```bash
   # Expected behavior:
   [GPU MINING] ✅ Template fetched successfully (height: X)
   # Silent mining for ~10 seconds (cache valid)
   [GPU MINING] 🔄 Cache expired (10s old), fetching new template...
   [GPU MINING] ✅ Template fetched successfully (height: X+1)
   ```

3. **If rate limited**, you should see:
   ```bash
   [GPU MINING] ⚠️ Rate limited - will reuse existing template if available
   [GPU MINING] ♻️ Reusing existing template to continue mining
   [GPU MINING] 📅 Extended cache TTL by 10s to avoid rate limit
   ```

4. **No more continuous 429 errors!** The miner will:
   - Continue mining with cached template during rate limits
   - Only fetch new templates when truly needed (cache expired AND not rate limited)
   - Wait appropriately between fetch attempts

## Rust Best Practices Applied

Following `/home/bob/Documents/rust-best-practices`:

1. **Pattern Matching** (Chapter 1.3): Used clear pattern matching for cache validity check instead of complex boolean logic
2. **Error Handling** (Chapter 1.3): Proper error differentiation without unwrap()
3. **Borrowing** (Chapter 1.1): Used `ref mut` for mutable borrow of cached_template
4. **Clear Intent**: Each code path has explicit logging to understand behavior

## Verification Steps

1. ✅ Code compiles without errors (`cargo check` passed)
2. ✅ Release build completed (`cargo build --release` successful)
3. ⏳ Manual testing needed - Start GPU mining and verify no 429 errors
4. ⏳ Monitor for 5+ minutes to ensure stable operation

## Summary

The fix addresses all three root causes of the rate limiting issue:
- Templates are preserved during rate limits
- Cache logic correctly determines when to fetch
- Rate limit errors are handled gracefully with fallback to existing templates

This should completely eliminate the continuous 429 errors and allow stable GPU mining operation.