# BTPC Supabase Setup

## Overview

This directory contains the Supabase local development setup for the BTPC project. Supabase provides a complete backend solution including:

- PostgreSQL database
- RESTful API (PostgREST)
- Authentication (GoTrue)
- Storage
- Realtime subscriptions
- Edge Functions
- Admin Dashboard (Studio)

## What's Included

### Database Schema

The initial migration (`migrations/20251018000000_initial_btpc_schema.sql`) sets up the following tables:

- **wallets** - Store wallet information
- **addresses** - Wallet addresses with labels
- **blocks** - Blockchain block data
- **transactions** - Transaction records
- **transaction_inputs** - Transaction input details
- **transaction_outputs** - Transaction output details
- **utxos** - Unspent transaction outputs
- **mining_stats** - Mining statistics and rewards
- **node_peers** - Network peer information
- **app_settings** - Application configuration

### Seed Data

The `seed.sql` file contains sample development data including:
- 3 development wallets
- Sample addresses
- Genesis block and first few blocks
- Sample transactions and UTXOs
- Mining statistics
- Node peers
- Application settings

### Edge Functions

The `functions/` directory contains serverless Edge Functions:
- `hello-world/` - Sample Edge Function demonstrating the basic structure

## Getting Started

### Prerequisites

- Docker installed and running
- Supabase CLI installed (version 2.51.0+)

### Starting Supabase

To start the local Supabase instance:

```bash
supabase start
```

This will:
1. Pull necessary Docker images (first time only)
2. Start all Supabase services
3. Apply migrations
4. Seed the database
5. Display access URLs and credentials

### Access URLs

Once started, you can access:

- **API URL**: http://127.0.0.1:54321
- **GraphQL URL**: http://127.0.0.1:54321/graphql/v1
- **S3 Storage URL**: http://127.0.0.1:54321/storage/v1/s3
- **Database URL**: postgresql://postgres:postgres@127.0.0.1:54322/postgres
- **Studio URL**: http://127.0.0.1:54323
- **Mailpit URL**: http://127.0.0.1:54324 (for testing emails)

### Accessing the Database

You can connect to the database using any PostgreSQL client:

```bash
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres
```

Or use the Supabase Studio dashboard at http://127.0.0.1:54323

### API Keys

When Supabase starts, it provides two keys:
- **Publishable key** (sb_publishable_*): Safe to use in client applications
- **Secret key** (sb_secret_*): For server-side operations only

These keys are automatically generated and displayed when you run `supabase start`.

## Common Commands

### Stop Supabase

```bash
supabase stop
```

### Reset Database

To reset the database and reapply migrations:

```bash
supabase db reset
```

### Create New Migration

```bash
supabase migration new <migration_name>
```

### Deploy Edge Function

```bash
supabase functions deploy <function_name>
```

### View Logs

```bash
supabase logs
```

### Check Status

```bash
supabase status
```

## Development Workflow

1. **Start Supabase**: `supabase start`
2. **Make schema changes**: Create new migration files
3. **Test changes**: Use `supabase db reset` to test migrations
4. **Build features**: Use the API, Storage, or Edge Functions
5. **Stop Supabase**: `supabase stop` when done

## Database Migrations

Migrations are stored in `supabase/migrations/` and are applied in order based on their timestamp prefix.

To create a new migration:

```bash
supabase migration new my_new_feature
```

Edit the generated SQL file, then reset the database to apply:

```bash
supabase db reset
```

## Edge Functions

Edge Functions are TypeScript/JavaScript functions that run on Deno.

To create a new function:

```bash
supabase functions new my-function
```

To deploy locally for testing:

```bash
supabase functions serve my-function
```

## Environment Variables

Environment variables are stored in `.env.local` (not committed to git).

Key variables:
- `SUPABASE_URL` - API URL
- `SUPABASE_ANON_KEY` - Publishable key
- `SUPABASE_SERVICE_ROLE_KEY` - Secret key
- `DATABASE_URL` - Direct database connection
- `BTPC_NETWORK` - BTPC network type (testnet/mainnet)
- `BTPC_NODE_URL` - BTPC node URL
- `BTPC_NODE_RPC_USER` - RPC username
- `BTPC_NODE_RPC_PASSWORD` - RPC password

## Integration with BTPC Desktop App

The Supabase backend can be used by the BTPC desktop app for:

1. **Persistent Storage**: Store wallet data, transactions, and settings
2. **Sync**: Synchronize data across devices
3. **Analytics**: Track mining statistics and performance
4. **Real-time Updates**: Get live updates on new blocks and transactions

To connect from the desktop app, use the Supabase JavaScript client:

```javascript
import { createClient } from '@supabase/supabase-js'

const supabase = createClient(
  process.env.SUPABASE_URL,
  process.env.SUPABASE_ANON_KEY
)

// Query example
const { data, error } = await supabase
  .from('wallets')
  .select('*')
  .eq('is_active', true)
```

## Troubleshooting

### Port Conflicts

If you get port conflict errors, check if another service is using the required ports:
- 54321 (API)
- 54322 (Database)
- 54323 (Studio)
- 54324 (Mailpit)

### Docker Issues

Ensure Docker is running:

```bash
docker ps
```

### Reset Everything

To completely reset Supabase:

```bash
supabase stop
supabase start
```

## Next Steps

1. Explore the Studio dashboard at http://127.0.0.1:54323
2. Review the database schema in the Table Editor
3. Test the API using the API docs in Studio
4. Create custom Edge Functions for your use case
5. Integrate with the BTPC desktop application

## Resources

- [Supabase Documentation](https://supabase.com/docs)
- [Local Development Guide](https://supabase.com/docs/guides/local-development)
- [CLI Reference](https://supabase.com/docs/guides/local-development/cli)
- [JavaScript Client](https://supabase.com/docs/reference/javascript)