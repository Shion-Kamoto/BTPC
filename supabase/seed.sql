-- BTPC Development Seed Data
-- This file contains sample data for local development and testing

-- Insert sample wallets
INSERT INTO wallets (name, address, balance, is_active, metadata) VALUES
    ('Development Wallet 1', 'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', 1000000000, true, '{"type": "development", "purpose": "testing"}'),
    ('Development Wallet 2', 'btpc1qr583w2swedy2acd7rung055k8t3n7udp89x4ha', 500000000, true, '{"type": "development", "purpose": "mining"}'),
    ('Test Wallet', 'btpc1qfwqe4u9g3fj6g6f7q9xqy9xqy9xqy9xqy9xqy9', 250000000, true, '{"type": "test", "purpose": "integration"}');

-- Insert sample addresses (linked to first wallet)
INSERT INTO addresses (wallet_id, address, label, is_change, index)
SELECT
    w.id,
    'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
    'Main Address',
    false,
    0
FROM wallets w WHERE w.name = 'Development Wallet 1';

INSERT INTO addresses (wallet_id, address, label, is_change, index)
SELECT
    w.id,
    'btpc1qzy3kgdygjrsqtzq2n0yrf2493p83kkfjhx1abc',
    'Change Address 1',
    true,
    1
FROM wallets w WHERE w.name = 'Development Wallet 1';

-- Insert genesis block
INSERT INTO blocks (height, hash, previous_hash, timestamp, nonce, difficulty, merkle_root, miner_address, reward, transaction_count, size) VALUES
    (0, '000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f', '0000000000000000000000000000000000000000000000000000000000000000', 1231006505, 2083236893, 1, '4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b', 'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', 5000000000, 1, 285);

-- Insert some sample blocks
INSERT INTO blocks (height, hash, previous_hash, timestamp, nonce, difficulty, merkle_root, miner_address, reward, transaction_count, size) VALUES
    (1, '00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048', '000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f', 1231469665, 2573394689, 1, '0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098', 'btpc1qr583w2swedy2acd7rung055k8t3n7udp89x4ha', 5000000000, 1, 215),
    (2, '000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd', '00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048', 1231469744, 1639830024, 1, '9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5', 'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', 5000000000, 1, 215);

-- Insert sample transactions
INSERT INTO transactions (txid, block_hash, block_height, timestamp, version, fee, size, confirmations, is_pending) VALUES
    ('4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b', '000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f', 0, 1231006505, 1, 0, 204, 3, false),
    ('0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098', '00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048', 1, 1231469665, 1, 0, 134, 2, false),
    ('9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5', '000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd', 2, 1231469744, 1, 0, 134, 1, false);

-- Insert sample transaction outputs (coinbase transactions)
INSERT INTO transaction_outputs (transaction_id, output_index, value, script_pubkey, address, is_spent)
SELECT t.id, 0, 5000000000, '76a914...88ac', 'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', false
FROM transactions t WHERE t.txid = '4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b';

INSERT INTO transaction_outputs (transaction_id, output_index, value, script_pubkey, address, is_spent)
SELECT t.id, 0, 5000000000, '76a914...88ac', 'btpc1qr583w2swedy2acd7rung055k8t3n7udp89x4ha', false
FROM transactions t WHERE t.txid = '0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098';

-- Insert sample UTXOs
INSERT INTO utxos (wallet_id, txid, output_index, address, value, script_pubkey, confirmations)
SELECT
    w.id,
    '4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b',
    0,
    'btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh',
    5000000000,
    '76a914...88ac',
    3
FROM wallets w WHERE w.name = 'Development Wallet 1';

INSERT INTO utxos (wallet_id, txid, output_index, address, value, script_pubkey, confirmations)
SELECT
    w.id,
    '0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098',
    0,
    'btpc1qr583w2swedy2acd7rung055k8t3n7udp89x4ha',
    5000000000,
    '76a914...88ac',
    2
FROM wallets w WHERE w.name = 'Development Wallet 2';

-- Insert sample mining statistics
INSERT INTO mining_stats (wallet_id, block_height, block_hash, difficulty, hash_rate, shares_submitted, shares_accepted, reward, is_successful)
SELECT
    w.id,
    1,
    '00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048',
    1,
    1000000,
    100,
    95,
    5000000000,
    true
FROM wallets w WHERE w.name = 'Development Wallet 2';

-- Insert sample node peers
INSERT INTO node_peers (peer_id, address, port, is_active, version, user_agent, latency_ms) VALUES
    ('peer1', '192.168.1.100', 8333, true, '1.0.0', 'BTPC/1.0.0', 45),
    ('peer2', '192.168.1.101', 8333, true, '1.0.0', 'BTPC/1.0.0', 52),
    ('peer3', '10.0.0.50', 8333, false, '0.9.9', 'BTPC/0.9.9', 150);

-- Insert sample app settings
INSERT INTO app_settings (key, value) VALUES
    ('network', '{"type": "testnet", "name": "BTPC Testnet"}'),
    ('mining', '{"enabled": false, "threads": 4, "algorithm": "SHA512"}'),
    ('node', '{"mode": "local", "port": 8333, "rpc_port": 8332}'),
    ('ui', '{"theme": "dark", "language": "en", "currency": "USD"}');