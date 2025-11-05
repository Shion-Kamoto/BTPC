-- BTPC Initial Schema Migration
-- This creates the core tables for the BTPC blockchain desktop application

-- Enable necessary extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Wallets table
CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    address TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    balance BIGINT DEFAULT 0,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Addresses table
CREATE TABLE IF NOT EXISTS addresses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID REFERENCES wallets(id) ON DELETE CASCADE,
    address TEXT UNIQUE NOT NULL,
    label TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_change BOOLEAN DEFAULT false,
    index INTEGER NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Blocks table
CREATE TABLE IF NOT EXISTS blocks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    height BIGINT UNIQUE NOT NULL,
    hash TEXT UNIQUE NOT NULL,
    previous_hash TEXT,
    timestamp BIGINT NOT NULL,
    nonce BIGINT NOT NULL,
    difficulty BIGINT NOT NULL,
    merkle_root TEXT NOT NULL,
    miner_address TEXT,
    reward BIGINT DEFAULT 0,
    transaction_count INTEGER DEFAULT 0,
    size INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    txid TEXT UNIQUE NOT NULL,
    block_hash TEXT,
    block_height BIGINT,
    timestamp BIGINT NOT NULL,
    version INTEGER DEFAULT 1,
    fee BIGINT DEFAULT 0,
    size INTEGER DEFAULT 0,
    confirmations INTEGER DEFAULT 0,
    is_pending BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Transaction inputs table
CREATE TABLE IF NOT EXISTS transaction_inputs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    previous_txid TEXT NOT NULL,
    previous_output_index INTEGER NOT NULL,
    script_sig TEXT,
    sequence INTEGER,
    witness JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Transaction outputs table
CREATE TABLE IF NOT EXISTS transaction_outputs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    output_index INTEGER NOT NULL,
    value BIGINT NOT NULL,
    script_pubkey TEXT NOT NULL,
    address TEXT,
    is_spent BOOLEAN DEFAULT false,
    spent_by_txid TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(transaction_id, output_index)
);

-- UTXOs (Unspent Transaction Outputs) table
CREATE TABLE IF NOT EXISTS utxos (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID REFERENCES wallets(id) ON DELETE CASCADE,
    txid TEXT NOT NULL,
    output_index INTEGER NOT NULL,
    address TEXT NOT NULL,
    value BIGINT NOT NULL,
    script_pubkey TEXT NOT NULL,
    confirmations INTEGER DEFAULT 0,
    is_locked BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(txid, output_index)
);

-- Mining statistics table
CREATE TABLE IF NOT EXISTS mining_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID REFERENCES wallets(id) ON DELETE CASCADE,
    block_height BIGINT,
    block_hash TEXT,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    difficulty BIGINT NOT NULL,
    hash_rate BIGINT DEFAULT 0,
    shares_submitted INTEGER DEFAULT 0,
    shares_accepted INTEGER DEFAULT 0,
    reward BIGINT DEFAULT 0,
    is_successful BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Node peers table
CREATE TABLE IF NOT EXISTS node_peers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    peer_id TEXT UNIQUE NOT NULL,
    address TEXT NOT NULL,
    port INTEGER NOT NULL,
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    version TEXT,
    user_agent TEXT,
    latency_ms INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Application settings table
CREATE TABLE IF NOT EXISTS app_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key TEXT UNIQUE NOT NULL,
    value JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_addresses_wallet_id ON addresses(wallet_id);
CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses(address);
CREATE INDEX IF NOT EXISTS idx_blocks_height ON blocks(height);
CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash);
CREATE INDEX IF NOT EXISTS idx_transactions_txid ON transactions(txid);
CREATE INDEX IF NOT EXISTS idx_transactions_block_hash ON transactions(block_hash);
CREATE INDEX IF NOT EXISTS idx_transactions_block_height ON transactions(block_height);
CREATE INDEX IF NOT EXISTS idx_transaction_inputs_transaction_id ON transaction_inputs(transaction_id);
CREATE INDEX IF NOT EXISTS idx_transaction_outputs_transaction_id ON transaction_outputs(transaction_id);
CREATE INDEX IF NOT EXISTS idx_transaction_outputs_address ON transaction_outputs(address);
CREATE INDEX IF NOT EXISTS idx_utxos_wallet_id ON utxos(wallet_id);
CREATE INDEX IF NOT EXISTS idx_utxos_address ON utxos(address);
CREATE INDEX IF NOT EXISTS idx_utxos_txid ON utxos(txid);
CREATE INDEX IF NOT EXISTS idx_mining_stats_wallet_id ON mining_stats(wallet_id);
CREATE INDEX IF NOT EXISTS idx_mining_stats_timestamp ON mining_stats(timestamp);
CREATE INDEX IF NOT EXISTS idx_node_peers_peer_id ON node_peers(peer_id);

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at triggers
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_transactions_updated_at BEFORE UPDATE ON transactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_transaction_outputs_updated_at BEFORE UPDATE ON transaction_outputs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_utxos_updated_at BEFORE UPDATE ON utxos
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_app_settings_updated_at BEFORE UPDATE ON app_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();