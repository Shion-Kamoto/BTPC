# BIP39 Mnemonic Wallet Recovery - User Guide

**Feature 008: Deterministic Wallet Recovery**
**Version**: 1.0.0
**Last Updated**: 2025-11-06
**Status**: Production Ready

---

## What is BIP39 Mnemonic Recovery?

BIP39 mnemonic recovery allows you to back up and restore your BTPC wallet using a **24-word recovery phrase**. This phrase is human-readable and can be written down on paper for secure offline storage.

### Key Benefits

1. **Cross-Device Recovery**: Restore your wallet on any device with just your 24 words
2. **Deterministic Keys**: Same phrase always generates the same keys (mathematically guaranteed)
3. **Offline Backup**: No need for encrypted files - just write down 24 words
4. **Universal Standard**: Compatible with BIP39 standard used by many cryptocurrencies

---

## Creating a New Wallet with Mnemonic

### Step 1: Open Wallet Manager

1. Launch BTPC Desktop App
2. Navigate to **Wallet Manager** tab
3. Click **"Create New Wallet"** button

### Step 2: Generate Mnemonic

1. The app will generate a random 24-word mnemonic phrase
2. **CRITICAL**: Write down all 24 words in exact order on paper
3. Store this paper in a secure location (safe, bank vault, etc.)
4. **NEVER** share your mnemonic with anyone or store it digitally

### Step 3: Verify Your Mnemonic

1. The app will ask you to verify some words from your phrase
2. Enter the requested words exactly as shown
3. This ensures you wrote them down correctly

### Step 4: Set Password (Optional)

1. You can add an optional passphrase for additional security
2. **WARNING**: If you use a passphrase, you MUST remember it
3. Without the passphrase, your mnemonic alone won't recover your wallet
4. Leave blank if you don't want to use a passphrase

### Step 5: Wallet Created

1. Your wallet is now created and ready to use
2. You'll see a **"V2 BIP39"** badge indicating it's a deterministic wallet
3. Your wallet address will be displayed
4. You can now receive and send BTPC

---

## Recovering an Existing Wallet

### Step 1: Open Wallet Manager

1. Launch BTPC Desktop App
2. Navigate to **Wallet Manager** tab
3. Click **"Recover Wallet from Mnemonic"** button

### Step 2: Enter Your Mnemonic

1. Type or paste your 24-word mnemonic phrase
2. Words can be separated by spaces, tabs, or newlines
3. Case doesn't matter (ABANDON = abandon = Abandon)
4. Extra whitespace is ignored

### Step 3: Enter Passphrase (If Used)

1. If you used a passphrase when creating the wallet, enter it exactly
2. **CRITICAL**: Even one character wrong will generate a different wallet
3. Leave blank if you didn't use a passphrase

### Step 4: Recovery Complete

1. The app will regenerate your wallet from the mnemonic
2. Your original address will reappear
3. Your transaction history will be re-synced from the blockchain
4. You can now access your funds

---

## Security Best Practices

### Mnemonic Storage

✅ **DO:**
- Write your mnemonic on paper with permanent ink
- Store in a fireproof/waterproof safe
- Consider splitting across multiple secure locations
- Make multiple backup copies stored separately
- Use a metal backup device for extreme durability

❌ **DON'T:**
- Store on your computer or phone (screenshots, text files, etc.)
- Email or message your mnemonic to anyone
- Store in cloud services (Google Drive, Dropbox, etc.)
- Take photos of your mnemonic
- Share with anyone, even "tech support"

### Passphrase Usage

✅ **DO:**
- Use a passphrase if you store your mnemonic in less secure locations
- Remember that passphrase + mnemonic = your wallet
- Write down whether you used a passphrase (but NOT the passphrase itself)

❌ **DON'T:**
- Use a passphrase you might forget
- Store passphrase next to your mnemonic
- Use personal info as passphrase (birthday, name, etc.)

### Verification

✅ **DO:**
- Test recovery on a fresh device before sending large amounts
- Verify you wrote down all 24 words correctly
- Check the word order is correct (word #1, #2, etc.)

---

## Troubleshooting

### "Invalid mnemonic phrase" Error

**Possible Causes:**
1. Missing a word (need exactly 24 words)
2. Misspelled word (check against BIP39 wordlist)
3. Words in wrong order
4. Using a word not in BIP39 wordlist

**Solution:**
- Count your words - must be exactly 24
- Check spelling carefully
- Verify word order matches your written backup
- Try entering words one at a time to find the problematic word

### "Checksum verification failed" Error

**Possible Causes:**
1. Last word is incorrect (word #24 contains checksum)
2. One or more words were transcribed incorrectly

**Solution:**
- Double-check the last word (word #24)
- Try variations of the last word if you're unsure
- Verify all other words are correct first
- If you have partial info, you may need professional recovery services

### Different Address After Recovery

**Possible Causes:**
1. Used different passphrase than original
2. Recovered wrong wallet (different mnemonic)
3. Using different network (mainnet vs testnet)

**Solution:**
- Try recovering without passphrase (if you might not have used one)
- Try common passphrase variations you might have used
- Verify you're using the correct mnemonic
- Check which network you're connected to

### Wallet Version Shows "V1 Legacy"

This means your wallet was created before BIP39 support. To upgrade:

1. Create a new V2 BIP39 wallet (generates new mnemonic)
2. Transfer all funds from V1 to V2 wallet
3. Back up your V2 mnemonic phrase
4. Keep V1 backup until transfer confirmed

**Note**: You cannot generate a mnemonic for existing V1 wallets (they weren't created deterministically).

---

## Understanding Wallet Versions

The BTPC Desktop App displays wallet version badges:

### V2 BIP39 (Green Badge)
- **Created**: Feature 008 onwards (2025-11-06+)
- **Backup**: 24-word mnemonic phrase
- **Recovery**: Full cross-device recovery supported
- **Deterministic**: Yes (same mnemonic = same keys)

### V1 Legacy (Gray Badge)
- **Created**: Before Feature 008
- **Backup**: Encrypted .dat file only
- **Recovery**: File-based recovery only
- **Deterministic**: No (random key generation)

---

## Example Walkthrough

### Scenario: Creating and Recovering a Wallet

1. **Day 1: Create Wallet**
   - Open BTPC Desktop App
   - Click "Create New Wallet"
   - Mnemonic generated: "abandon abandon abandon ... art" (24 words)
   - Write down all 24 words on paper
   - No passphrase used
   - Wallet created with address: btpc1q7j5t2x...

2. **Day 30: Send BTPC**
   - Receive 10 BTPC to address
   - Send 3 BTPC to friend
   - Balance: 7 BTPC remaining

3. **Day 60: Computer Crashes**
   - Hard drive fails completely
   - Need to recover wallet on new computer

4. **Day 61: Recovery**
   - Install BTPC Desktop App on new computer
   - Click "Recover Wallet from Mnemonic"
   - Enter 24 words: "abandon abandon abandon ... art"
   - No passphrase (leave blank)
   - Wallet recovered!
   - Address matches: btpc1q7j5t2x...
   - Balance syncs: 7 BTPC ✓

---

## FAQ

### Q: Can I use a 12-word mnemonic?

**A**: No, BTPC only supports 24-word mnemonics for maximum security.

### Q: What if I lose one word from my mnemonic?

**A**: Recovery is extremely difficult but possible with specialized software (try all 2048 possibilities for that position). This could take hours to days depending on which word is missing.

### Q: Can I change my mnemonic after creating a wallet?

**A**: No, each mnemonic is tied to specific keys. To get a new mnemonic, create a new wallet and transfer funds.

### Q: Is my mnemonic the same as my password?

**A**: No. Your mnemonic recovers your wallet. Your password encrypts the wallet file. You need the mnemonic to recover on a new device.

### Q: Can someone steal my funds if they find my mnemonic?

**A**: YES. Anyone with your 24 words can fully access your wallet. Guard it like cash.

### Q: How long does recovery take?

**A**: Wallet recovery is instant (2-3 seconds). Blockchain sync may take longer depending on transaction history.

### Q: Can I recover on a different cryptocurrency wallet?

**A**: No. BTPC uses BIP39 for mnemonics but generates ML-DSA (post-quantum) keys, which are unique to BTPC. Standard Bitcoin/Ethereum wallets won't work.

---

## Technical Details (Advanced Users)

- **Standard**: BIP39 (Bitcoin Improvement Proposal 39)
- **Entropy**: 256 bits (24 words)
- **Checksum**: 8 bits (encoded in last word)
- **Wordlist**: English BIP39 wordlist (2048 words)
- **Key Derivation**: PBKDF2-HMAC-SHA512 (2048 rounds)
- **Seed Expansion**: SHAKE256 XOF (32 bytes → ML-DSA key material)
- **Signature Algorithm**: ML-DSA (Dilithium5, post-quantum)
- **Key Size**: 4000 bytes private, 1952 bytes public

---

## Support

If you encounter issues not covered in this guide:

1. Check the **Troubleshooting** section above
2. Review error messages carefully
3. Consult the Developer Guide for technical details
4. Report bugs at: [BTPC GitHub Issues](https://github.com/btpc/btpc/issues)

**NEVER share your mnemonic when asking for support!**

---

*This guide is part of Feature 008: BIP39 Deterministic Wallet Recovery*
*For technical implementation details, see DEVELOPER_GUIDE.md*