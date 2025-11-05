#!/usr/bin/env python3
"""
Orphaned UTXO Cleanup Script
Implements the same logic as the Rust orphaned_utxo_cleaner module
"""

import json
import os
import shutil
from pathlib import Path
from datetime import datetime

def clean_orphaned_utxos(utxo_file, wallets_dir, dry_run=True):
    """Clean orphaned UTXOs that don't belong to any current wallet"""

    # Read UTXO file
    if not utxo_file.exists():
        raise FileNotFoundError(f"UTXO file not found: {utxo_file}")

    print(f"📁 Reading UTXO file: {utxo_file}")
    with open(utxo_file, 'r') as f:
        utxos = json.load(f)

    # Build set of wallet identifiers (public keys + addresses)
    wallet_identifiers = set()

    print(f"📁 Reading wallet files from: {wallets_dir}")
    for filename in os.listdir(wallets_dir):
        if filename.startswith('wallet_') and filename.endswith('.json'):
            wallet_path = wallets_dir / filename
            with open(wallet_path, 'r') as f:
                wallet_data = json.load(f)

                if 'public_key' in wallet_data:
                    wallet_identifiers.add(wallet_data['public_key'])
                if 'address' in wallet_data:
                    wallet_identifiers.add(wallet_data['address'])

    print(f"📊 Found {len(wallet_identifiers)} wallet identifiers (public keys + addresses)")

    # Separate UTXOs into owned and orphaned
    owned_utxos = []
    orphaned_utxos = []
    orphaned_value_credits = 0

    for utxo in utxos:
        address = utxo.get('address', '')
        if address in wallet_identifiers:
            owned_utxos.append(utxo)
        else:
            # This is an orphaned UTXO
            value = utxo.get('value_credits', 0)
            orphaned_value_credits += value
            orphaned_utxos.append(utxo)

    # Create report
    report = {
        'total_utxos': len(owned_utxos) + len(orphaned_utxos),
        'owned_utxos': len(owned_utxos),
        'orphaned_utxos': len(orphaned_utxos),
        'orphaned_value_credits': orphaned_value_credits,
        'orphaned_value_btp': orphaned_value_credits / 100_000_000.0
    }

    # Print report
    print("\n📋 Orphaned UTXO Report:")
    print(f"  Total UTXOs: {report['total_utxos']}")
    print(f"  ✅ Owned UTXOs (belong to current wallets): {report['owned_utxos']}")
    print(f"  ❌ Orphaned UTXOs (no matching wallet): {report['orphaned_utxos']}")
    print(f"  💰 Orphaned value: {report['orphaned_value_credits']} credits ({report['orphaned_value_btp']:.8f} BTP)")

    if not dry_run and report['orphaned_utxos'] > 0:
        # Create backup
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        backup_file = utxo_file.parent / f"wallet_utxos.json.orphan_backup_{timestamp}"
        shutil.copy2(utxo_file, backup_file)
        print(f"\n📦 Created backup: {backup_file}")

        # Write cleaned UTXO file with only owned UTXOs
        with open(utxo_file, 'w') as f:
            json.dump(owned_utxos, f, indent=2)

        print(f"✅ Removed {report['orphaned_utxos']} orphaned UTXOs from {utxo_file}")
        print(f"⚠️  WARNING: Removed {report['orphaned_value_btp']:.8f} BTP ({report['orphaned_value_credits']} credits) in orphaned funds")

    elif report['orphaned_utxos'] > 0:
        print(f"\n🔍 DRY RUN: Would remove {report['orphaned_utxos']} orphaned UTXOs")
        print("   Run with dry_run=False to actually remove them")
    else:
        print("\n✅ No orphaned UTXOs found - all UTXOs belong to current wallets")

    return report


def main():
    utxo_file = Path("/home/bob/.btpc/data/wallet/wallet_utxos.json")
    wallets_dir = Path("/home/bob/.btpc/wallets")

    print("🧹 Orphaned UTXO Cleanup")
    print("=" * 50)
    print()

    # Phase 1: Dry run
    print("Phase 1: DRY RUN (preview only)")
    print("-" * 50)
    report = clean_orphaned_utxos(utxo_file, wallets_dir, dry_run=True)
    print(f"\n✅ Dry run completed successfully")
    print(f"   Would remove {report['orphaned_utxos']} orphaned UTXOs totaling {report['orphaned_value_btp']:.8f} BTP")

    # Ask for confirmation
    print("\n" + "=" * 50)
    response = input("\nDo you want to proceed with the cleanup? (yes/no): ")

    if response.strip().lower() != 'yes':
        print("❌ Cleanup cancelled by user")
        return

    # Phase 2: Execute cleanup
    print("\n\nPhase 2: EXECUTE CLEANUP")
    print("-" * 50)
    report = clean_orphaned_utxos(utxo_file, wallets_dir, dry_run=False)

    print("\n" + "=" * 50)
    print("✅ Cleanup completed successfully!")
    print(f"   Removed {report['orphaned_utxos']} orphaned UTXOs totaling {report['orphaned_value_btp']:.8f} BTP")
    print(f"   Remaining UTXOs: {report['owned_utxos']} (all owned by current wallets)")
    print("=" * 50)


if __name__ == '__main__':
    main()