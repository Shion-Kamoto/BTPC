#!/usr/bin/env python3
"""
Update the wallets_metadata.json to include testingW2
"""

import json
import os
from datetime import datetime, timezone

def main():
    metadata_file = "/home/bob/.btpc/wallets/wallets_metadata.json"
    wallet_file = "/home/bob/.btpc/wallets/6e78ab5a-9dc0-4b28-aeb0-c1160a9e119f.json"

    # Read existing metadata
    with open(metadata_file, 'r') as f:
        metadata = json.load(f)

    # Read wallet data
    with open(wallet_file, 'r') as f:
        wallet_data = json.load(f)

    wallet_id = wallet_data['id']

    # Create metadata entry for testingW2
    now = datetime.now(timezone.utc).isoformat()

    metadata[wallet_id] = {
        "id": wallet_id,
        "nickname": wallet_data['nickname'],
        "address": wallet_data['address'],
        "file_path": wallet_file,
        "created_at": wallet_data['created_at'],
        "last_accessed": now,
        "metadata": wallet_data['metadata'],
        "cached_balance_credits": wallet_data.get('cached_balance_credits', 0),
        "cached_balance_btp": wallet_data.get('cached_balance_btp', 0.0),
        "balance_updated_at": wallet_data.get('balance_updated_at', now),
        "is_default": wallet_data.get('is_default', False),
        "source": {
            "CreatedNew": {
                "version": "1.0.0"
            }
        }
    }

    # Write updated metadata
    with open(metadata_file, 'w') as f:
        json.dump(metadata, f, indent=2)

    print(f"✅ Added testingW2 to wallets metadata")
    print(f"   Wallet ID: {wallet_id}")
    print(f"   Address: {wallet_data['address']}")

if __name__ == "__main__":
    main()
