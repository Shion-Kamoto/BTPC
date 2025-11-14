---
description: Manage Supabase local development environment for BTPC
---

You are helping manage the Supabase local development environment for the BTPC blockchain project.

# Context

The BTPC project uses Supabase as a local backend solution providing:
- PostgreSQL database with BTPC-specific schema (wallets, transactions, blocks, UTXOs, etc.)
- RESTful API via PostgREST
- Authentication via GoTrue
- Storage for files
- Edge Functions for serverless logic
- Real-time subscriptions
- Admin dashboard (Studio)

All Supabase files are located in `/home/bob/BTPC/BTPC/supabase/`.

# Available Operations

When the user invokes `/supabase`, determine their intent and perform the appropriate action:

## 1. Status Check (`/supabase status` or `/supabase`)
Check if Supabase is running and display connection information:
- Run `supabase status`
- Display all service URLs and credentials
- Check if services are healthy

## 2. Start Services (`/supabase start`)
Start the local Supabase development environment:
- Run `supabase start`
- Wait for all containers to start
- Display access URLs and credentials
- Inform user about Studio dashboard URL

## 3. Stop Services (`/supabase stop`)
Stop all Supabase services:
- Run `supabase stop`
- Confirm services stopped successfully

## 4. Reset Database (`/supabase reset`)
Reset the database and reapply all migrations:
- **WARNING**: This will delete all data
- Ask for confirmation first
- Run `supabase db reset`
- Confirm migrations applied and seed data loaded

## 5. Create Migration (`/supabase migration <name>`)
Create a new database migration:
- Run `supabase migration new <name>`
- Open the created migration file for editing
- Provide guidance on migration best practices

## 6. View Logs (`/supabase logs`)
Display Supabase service logs:
- Run `supabase logs`
- Help diagnose any issues

## 7. Database Shell (`/supabase db`)
Provide database connection information:
- Display the psql connection string
- Explain how to connect via psql or other clients
- Optionally run queries if user specifies them

## 8. Studio (`/supabase studio`)
Open or provide info about Supabase Studio:
- Display Studio URL (http://127.0.0.1:54323)
- Explain what can be done in Studio:
  - Browse tables and data
  - Run SQL queries
  - View API documentation
  - Manage authentication
  - Configure storage buckets

## 9. Edge Functions (`/supabase function <action> <name>`)
Manage Edge Functions:
- `create <name>`: Create new Edge Function
- `serve <name>`: Serve function locally for testing
- `deploy <name>`: Deploy function
- List existing functions

## 10. Backup/Export (`/supabase backup`)
Create a database backup:
- Use `pg_dump` to export the database
- Save to a timestamped file
- Provide restore instructions

## 11. Troubleshooting (`/supabase troubleshoot`)
Help diagnose common issues:
- Check if Docker is running
- Check port conflicts (54321-54324)
- Verify Supabase CLI version
- Check container health
- Provide solutions to common problems

# Guidelines

1. **Always check status first**: Before performing operations, check if Supabase is running
2. **Be informative**: Display URLs and credentials when starting services
3. **Confirm destructive actions**: Ask before resetting database or deleting data
4. **Provide context**: Explain what each operation does
5. **Handle errors gracefully**: If commands fail, provide troubleshooting steps
6. **Follow BTPC schema**: When creating migrations, follow the existing schema patterns

# Database Schema Reference

The BTPC database includes these main tables:
- `wallets` - Wallet information and balances
- `addresses` - Wallet addresses with labels
- `blocks` - Blockchain block data
- `transactions` - Transaction records
- `transaction_inputs` - Transaction inputs
- `transaction_outputs` - Transaction outputs
- `utxos` - Unspent transaction outputs
- `mining_stats` - Mining statistics and rewards
- `node_peers` - Network peer information
- `app_settings` - Application configuration

# Examples

User: `/supabase`
→ Check and display current status

User: `/supabase start`
→ Start all services and display access information

User: `/supabase migration add_wallet_metadata`
→ Create new migration for wallet metadata

User: `/supabase reset`
→ Ask confirmation, then reset database

User: `/supabase function create validate-transaction`
→ Create new Edge Function for transaction validation

# Important Notes

- Supabase runs in Docker containers
- Local development uses ports 54321-54324
- Database password is `postgres` for local development
- All data is ephemeral unless backed up
- Studio dashboard is at http://127.0.0.1:54323
- API is at http://127.0.0.1:54321

# Output Format

Always provide:
1. Clear status of the operation
2. Relevant URLs and credentials (when applicable)
3. Next steps or recommendations
4. Error messages and solutions (if issues occur)

Be concise but informative. Use the TodoWrite tool if performing multiple steps.