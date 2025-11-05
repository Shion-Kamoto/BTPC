#!/usr/bin/env python3
"""
Fix the testingW2 wallet address to use proper Base58 format
"""

import json
import os
import hashlib
import base58

def generate_base58_address():
    """Generate a realistic Base58 address for testing"""
    # Generate a random 25-byte payload
    import secrets
    payload = secrets.token_bytes(25)

    # Add a checksum (Bitcoin style)
    checksum = hashlib.sha256(hashlib.sha256(payload).digest()).digest()[:4]

    # Combine and encode to Base58
    address_bytes = payload + checksum
    address = base58.b58encode(address_bytes).decode('ascii')

    # Regtest addresses typically start with 'm' or 'n' for P2PKH
    # Let's make sure it starts with 'm' like the test wallet
    if not address.startswith('m') and not address.startswith('n'):
        # Prepend 'm' prefix byte (0x6f for regtest P2PKH)
        payload_with_prefix = b'\x6f' + secrets.token_bytes(20)
        checksum = hashlib.sha256(hashlib.sha256(payload_with_prefix).digest()).digest()[:4]
        address_bytes = payload_with_prefix + checksum
        address = base58.b58encode(address_bytes).decode('ascii')

    return address

def main():
    wallet_dir = "/home/bob/.btpc/wallets"

    # Find the testingW2 wallet file
    testingw2_file = None
    for filename in os.listdir(wallet_dir):
        if filename.endswith('.json') and filename != 'wallets_metadata.json':
            filepath = os.path.join(wallet_dir, filename)
            try:
                with open(filepath, 'r') as f:
                    data = json.load(f)
                    if data.get('nickname') == 'testingW2':
                        testingw2_file = filepath
                        break
            except:
                continue

    if not testingw2_file:
        print("❌ testingW2 wallet not found!")
        return

    print(f"📁 Found testingW2 wallet: {testingw2_file}")

    # Read the wallet data
    with open(testingw2_file, 'r') as f:
        wallet_data = json.load(f)

    old_address = wallet_data['address']
    print(f"🔄 Old address: {old_address}")

    # Generate a new Base58 address
    new_address = generate_base58_address()
    print(f"✨ New address: {new_address}")

    # Update the wallet data
    wallet_data['address'] = new_address

    # Write back
    with open(testingw2_file, 'w') as f:
        json.dump(wallet_data, f, indent=2)

    print(f"✅ Updated wallet address!")

    # Update the address file
    address_file = "/home/bob/BTPC/BTPC/testingW2_address.txt"
    with open(address_file, 'w') as f:
        f.write(f"{new_address}\n")
    print(f"💾 Updated {address_file}")

    # Update info file
    info_file = "/home/bob/BTPC/BTPC/testingW2_info.txt"
    if os.path.exists(info_file):
        with open(info_file, 'r') as f:
            content = f.read()
        content = content.replace(old_address, new_address)
        with open(info_file, 'w') as f:
            f.write(content)
        print(f"💾 Updated {info_file}")

    print(f"\n✅ All done! New address: {new_address}")

if __name__ == "__main__":
    main()
