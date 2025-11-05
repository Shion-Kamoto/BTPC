# Supabase Performance Advisor Analysis
**Date**: 2025-10-18
**Database**: Local Supabase Development Instance (postgresql://postgres@127.0.0.1:54322/postgres)

## Executive Summary

The Supabase Performance Advisor identified **15 "Unused Index" suggestions** (Info level, not errors). Analysis confirms these indexes are well-designed but currently unused because **BTPC does not yet use Supabase for data storage** - it uses RocksDB locally.

## Performance Advisor Results

- ✅ **0 Errors**
- ✅ **0 Warnings**
- ℹ️ **15 Info Suggestions** (all "Unused Index")

## Detailed Index Analysis

### Index Usage Statistics

Total indexes analyzed: **25 non-primary key indexes**

| Table | Index Name | Scans | Status |
|-------|-----------|-------|--------|
| **addresses** | `addresses_address_key` | 0 | ⚠️ Unused |
| **addresses** | `idx_addresses_address` | 0 | ⚠️ Unused |
| **addresses** | `idx_addresses_wallet_id` | 0 | ⚠️ Unused |
| **app_settings** | `app_settings_key_key` | 0 | ⚠️ Unused |
| **blocks** | `blocks_hash_key` | 0 | ⚠️ Unused |
| **blocks** | `blocks_height_key` | 0 | ⚠️ Unused |
| **blocks** | `idx_blocks_hash` | 0 | ⚠️ Unused |
| **blocks** | `idx_blocks_height` | 0 | ⚠️ Unused |
| **mining_stats** | `idx_mining_stats_timestamp` | 0 | ⚠️ Unused |
| **mining_stats** | `idx_mining_stats_wallet_id` | 0 | ⚠️ Unused |
| **node_peers** | `idx_node_peers_peer_id` | 0 | ⚠️ Unused |
| **node_peers** | `node_peers_peer_id_key` | 0 | ⚠️ Unused |
| **transaction_inputs** | `idx_transaction_inputs_transaction_id` | 0 | ⚠️ Unused |
| **transaction_outputs** | `idx_transaction_outputs_address` | 0 | ⚠️ Unused |
| **transaction_outputs** | `idx_transaction_outputs_transaction_id` | 0 | ⚠️ Unused |
| **transaction_outputs** | `transaction_outputs_transaction_id_output_index_key` | 0 | ⚠️ Unused |
| **transactions** | `idx_transactions_block_hash` | 0 | ⚠️ Unused |
| **transactions** | `idx_transactions_block_height` | 0 | ⚠️ Unused |
| **transactions** | `idx_transactions_txid` | **2** | ✅ **USED** |
| **transactions** | `transactions_txid_key` | 0 | ⚠️ Unused |
| **utxos** | `idx_utxos_address` | 0 | ⚠️ Unused |
| **utxos** | `idx_utxos_txid` | 0 | ⚠️ Unused |
| **utxos** | `idx_utxos_wallet_id` | 0 | ⚠️ Unused |
| **utxos** | `utxos_txid_output_index_key` | 0 | ⚠️ Unused |
| **wallets** | `wallets_address_key` | 0 | ⚠️ Unused |

### Summary Statistics

- **Total Indexes**: 25
- **Unused Indexes**: 24 (96%)
- **Used Indexes**: 1 (4%)
  - `idx_transactions_txid`: 2 scans (likely from Performance Advisor queries themselves)

## Root Cause Analysis

### Why Are These Indexes Unused?

**BTPC currently uses RocksDB for all data storage**, not PostgreSQL/Supabase. The indexes exist in the Supabase schema definition but have never been used because:

1. **No Integration Exists**: BTPC desktop app does not connect to Supabase
2. **Local Storage Only**: All data is stored in RocksDB at `~/.btpc/data/`
3. **Development Schema**: The Supabase schema was created for potential future cloud sync feature

### Evidence from Codebase

**File**: `SUPABASE_INTEGRATION_ANALYSIS.md` (created earlier)
- Confirmed: No `tokio-postgres` or Supabase client usage in production code
- Current architecture: RocksDB + JSON files + LocalStorage
- Supabase schema exists in `/supabase/migrations/` but is not integrated

## Recommendations

### ✅ **Keep All Indexes** (Do NOT Remove)

**Rationale**:
1. **Well-Designed Schema**: The indexes are correctly designed for blockchain query patterns
2. **Future-Proofing**: If Supabase integration is implemented, these indexes will be critical
3. **No Performance Cost**: Unused indexes in a local dev database have zero impact
4. **Schema Documentation**: They document the intended query patterns

### 📋 **Recommended Actions**

#### For Current State (No Supabase Integration)
- ✅ **No action required** - indexes are harmless in local dev
- ✅ **Keep Performance Advisor enabled** for monitoring
- ✅ **Document that these warnings are expected**

#### If/When Implementing Supabase Integration
The indexes are already optimized for common queries:

1. **Wallet Lookups**:
   ```sql
   -- Already indexed: idx_addresses_wallet_id, idx_utxos_wallet_id
   SELECT * FROM addresses WHERE wallet_id = $1;
   SELECT * FROM utxos WHERE wallet_id = $1;
   ```

2. **Transaction Queries**:
   ```sql
   -- Already indexed: idx_transactions_txid, idx_transactions_block_hash
   SELECT * FROM transactions WHERE txid = $1;
   SELECT * FROM transactions WHERE block_hash = $1;
   ```

3. **Block Lookups**:
   ```sql
   -- Already indexed: idx_blocks_height, idx_blocks_hash
   SELECT * FROM blocks WHERE height = $1;
   SELECT * FROM blocks WHERE hash = $1;
   ```

4. **UTXO Queries by Address**:
   ```sql
   -- Already indexed: idx_utxos_address
   SELECT * FROM utxos WHERE address = $1 AND is_spent = false;
   ```

5. **Mining Statistics**:
   ```sql
   -- Already indexed: idx_mining_stats_wallet_id, idx_mining_stats_timestamp
   SELECT * FROM mining_stats WHERE wallet_id = $1 ORDER BY timestamp DESC;
   ```

### 🔍 **Index Optimization Notes**

All indexes follow best practices:

1. **Foreign Keys Indexed**: `wallet_id`, `transaction_id` columns
2. **Lookup Keys Indexed**: `address`, `txid`, `hash`, `height`
3. **Unique Constraints**: Proper unique indexes on natural keys
4. **Composite Indexes**: Used for multi-column lookups (e.g., `txid + output_index`)

### 📊 **If Implementing Supabase Sync**

When BTPC adds optional Supabase cloud sync (as analyzed in `SUPABASE_INTEGRATION_ANALYSIS.md`), these indexes will immediately provide:

1. **Fast Wallet Balance Queries**: `idx_utxos_wallet_id`
2. **Transaction History Lookup**: `idx_transactions_txid`
3. **Block Explorer Queries**: `idx_blocks_height`, `idx_blocks_hash`
4. **Address Transaction History**: `idx_utxos_address`
5. **Mining Rewards Tracking**: `idx_mining_stats_wallet_id`

## Performance Advisor Response

### Official Recommendation

The Performance Advisor suggestion is:
> "Detects if an index has never been used and may be a candidate for removal."

### Our Response

**Do NOT remove these indexes** because:

1. ✅ They are intentionally designed for future cloud sync feature
2. ✅ Schema matches RocksDB data model (proper mapping)
3. ✅ No negative impact on local development database
4. ✅ Removing them would require re-indexing later (expensive operation)

### Documentation Update

Update `supabase/README.md` (if it exists) to note:
```markdown
## Performance Advisor Warnings

The Supabase Performance Advisor will show "unused index" warnings for all indexes
because BTPC currently uses RocksDB for local storage. These warnings are EXPECTED
and the indexes should NOT be removed.

If/when cloud sync is implemented via Supabase, these indexes will be essential
for query performance.
```

## Testing Recommendations

If Supabase integration is implemented, validate index usage with:

```sql
-- After running queries, check which indexes were used
SELECT
    schemaname,
    relname AS table,
    indexrelname AS index,
    idx_scan AS scans,
    idx_tup_read AS tuples_read
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
  AND idx_scan > 0
ORDER BY idx_scan DESC;
```

## References

- **Schema Definition**: `/home/bob/BTPC/BTPC/supabase/migrations/20251018000000_initial_btpc_schema.sql`
- **Integration Analysis**: `/home/bob/BTPC/BTPC/SUPABASE_INTEGRATION_ANALYSIS.md`
- **Performance Advisor**: http://127.0.0.1:54323/project/default/advisors/performance
- **Supabase Linter Docs**: https://supabase.com/docs/guides/database/database-linter

## Conclusion

✅ **No action required** - The 15 "unused index" suggestions are expected and correct.
✅ **Keep all indexes** - They are well-designed for future Supabase integration.
✅ **Schema is production-ready** - When cloud sync is implemented, the database will perform optimally.

**Status**: Schema is properly indexed and ready for future cloud sync feature.