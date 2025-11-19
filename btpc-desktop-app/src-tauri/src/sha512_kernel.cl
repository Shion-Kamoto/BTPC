// SHA-512 Mining Kernel for BTPC
// Implements SHA-512 hashing for proof-of-work mining
// Based on NIST FIPS 180-4 specification

// SHA-512 constants (first 64 bits of fractional parts of cube roots of first 80 primes)
__constant ulong K[80] = {
    0x428a2f98d728ae22UL, 0x7137449123ef65cdUL, 0xb5c0fbcfec4d3b2fUL, 0xe9b5dba58189dbbcUL,
    0x3956c25bf348b538UL, 0x59f111f1b605d019UL, 0x923f82a4af194f9bUL, 0xab1c5ed5da6d8118UL,
    0xd807aa98a3030242UL, 0x12835b0145706fbeUL, 0x243185be4ee4b28cUL, 0x550c7dc3d5ffb4e2UL,
    0x72be5d74f27b896fUL, 0x80deb1fe3b1696b1UL, 0x9bdc06a725c71235UL, 0xc19bf174cf692694UL,
    0xe49b69c19ef14ad2UL, 0xefbe4786384f25e3UL, 0x0fc19dc68b8cd5b5UL, 0x240ca1cc77ac9c65UL,
    0x2de92c6f592b0275UL, 0x4a7484aa6ea6e483UL, 0x5cb0a9dcbd41fbd4UL, 0x76f988da831153b5UL,
    0x983e5152ee66dfabUL, 0xa831c66d2db43210UL, 0xb00327c898fb213fUL, 0xbf597fc7beef0ee4UL,
    0xc6e00bf33da88fc2UL, 0xd5a79147930aa725UL, 0x06ca6351e003826fUL, 0x142929670a0e6e70UL,
    0x27b70a8546d22ffcUL, 0x2e1b21385c26c926UL, 0x4d2c6dfc5ac42aedUL, 0x53380d139d95b3dfUL,
    0x650a73548baf63deUL, 0x766a0abb3c77b2a8UL, 0x81c2c92e47edaee6UL, 0x92722c851482353bUL,
    0xa2bfe8a14cf10364UL, 0xa81a664bbc423001UL, 0xc24b8b70d0f89791UL, 0xc76c51a30654be30UL,
    0xd192e819d6ef5218UL, 0xd69906245565a910UL, 0xf40e35855771202aUL, 0x106aa07032bbd1b8UL,
    0x19a4c116b8d2d0c8UL, 0x1e376c085141ab53UL, 0x2748774cdf8eeb99UL, 0x34b0bcb5e19b48a8UL,
    0x391c0cb3c5c95a63UL, 0x4ed8aa4ae3418acbUL, 0x5b9cca4f7763e373UL, 0x682e6ff3d6b2b8a3UL,
    0x748f82ee5defb2fcUL, 0x78a5636f43172f60UL, 0x84c87814a1f0ab72UL, 0x8cc702081a6439ecUL,
    0x90befffa23631e28UL, 0xa4506cebde82bde9UL, 0xbef9a3f7b2c67915UL, 0xc67178f2e372532bUL,
    0xca273eceea26619cUL, 0xd186b8c721c0c207UL, 0xeada7dd6cde0eb1eUL, 0xf57d4f7fee6ed178UL,
    0x06f067aa72176fbaUL, 0x0a637dc5a2c898a6UL, 0x113f9804bef90daeUL, 0x1b710b35131c471bUL,
    0x28db77f523047d84UL, 0x32caab7b40c72493UL, 0x3c9ebe0a15c9bebcUL, 0x431d67c49c100d4cUL,
    0x4cc5d4becb3e42b6UL, 0x597f299cfc657e2aUL, 0x5fcb6fab3ad6faecUL, 0x6c44198c4a475817UL
};

// Initial hash values (first 64 bits of fractional parts of square roots of first 8 primes)
__constant ulong H0[8] = {
    0x6a09e667f3bcc908UL, 0xbb67ae8584caa73bUL, 0x3c6ef372fe94f82bUL, 0xa54ff53a5f1d36f1UL,
    0x510e527fade682d1UL, 0x9b05688c2b3e6c1fUL, 0x1f83d9abfb41bd6bUL, 0x5be0cd19137e2179UL
};

// SHA-512 rotate right
#define ROTR(x, n) (((x) >> (n)) | ((x) << (64 - (n))))

// SHA-512 functions
#define CH(x, y, z)  (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define SIGMA0(x)    (ROTR(x, 28) ^ ROTR(x, 34) ^ ROTR(x, 39))
#define SIGMA1(x)    (ROTR(x, 14) ^ ROTR(x, 18) ^ ROTR(x, 41))
#define sigma0(x)    (ROTR(x, 1) ^ ROTR(x, 8) ^ ((x) >> 7))
#define sigma1(x)    (ROTR(x, 19) ^ ROTR(x, 61) ^ ((x) >> 6))

/**
 * Serialize BlockHeader to bytes for hashing
 *
 * BlockHeader structure:
 * - version: u32 (4 bytes)
 * - prev_hash: [u8; 64] (64 bytes)
 * - merkle_root: [u8; 64] (64 bytes)
 * - timestamp: u64 (8 bytes)
 * - bits: u32 (4 bytes)
 * - nonce: u32 (4 bytes)
 * Total: 148 bytes
 */
void serialize_header(
    uint version,
    __global const uchar *prev_hash,     // 64 bytes
    __global const uchar *merkle_root,   // 64 bytes
    ulong timestamp,
    uint bits,
    uint nonce,
    uchar *output                        // 148 bytes output
) {
    int offset = 0;

    // version (4 bytes, little-endian)
    output[offset++] = (uchar)(version);
    output[offset++] = (uchar)(version >> 8);
    output[offset++] = (uchar)(version >> 16);
    output[offset++] = (uchar)(version >> 24);

    // prev_hash (64 bytes)
    for (int i = 0; i < 64; i++) {
        output[offset++] = prev_hash[i];
    }

    // merkle_root (64 bytes)
    for (int i = 0; i < 64; i++) {
        output[offset++] = merkle_root[i];
    }

    // timestamp (8 bytes, little-endian)
    output[offset++] = (uchar)(timestamp);
    output[offset++] = (uchar)(timestamp >> 8);
    output[offset++] = (uchar)(timestamp >> 16);
    output[offset++] = (uchar)(timestamp >> 24);
    output[offset++] = (uchar)(timestamp >> 32);
    output[offset++] = (uchar)(timestamp >> 40);
    output[offset++] = (uchar)(timestamp >> 48);
    output[offset++] = (uchar)(timestamp >> 56);

    // bits (4 bytes, little-endian)
    output[offset++] = (uchar)(bits);
    output[offset++] = (uchar)(bits >> 8);
    output[offset++] = (uchar)(bits >> 16);
    output[offset++] = (uchar)(bits >> 24);

    // nonce (4 bytes, little-endian)
    output[offset++] = (uchar)(nonce);
    output[offset++] = (uchar)(nonce >> 8);
    output[offset++] = (uchar)(nonce >> 16);
    output[offset++] = (uchar)(nonce >> 24);
}

/**
 * SHA-512 compression function
 */
void sha512_transform(ulong *state, const uchar *block) {
    ulong W[80];
    ulong a, b, c, d, e, f, g, h;
    ulong T1, T2;

    // Prepare message schedule (first 16 words are from input)
    for (int i = 0; i < 16; i++) {
        W[i] = ((ulong)block[i*8 + 0] << 56) |
               ((ulong)block[i*8 + 1] << 48) |
               ((ulong)block[i*8 + 2] << 40) |
               ((ulong)block[i*8 + 3] << 32) |
               ((ulong)block[i*8 + 4] << 24) |
               ((ulong)block[i*8 + 5] << 16) |
               ((ulong)block[i*8 + 6] << 8) |
               ((ulong)block[i*8 + 7]);
    }

    // Extend message schedule
    for (int i = 16; i < 80; i++) {
        W[i] = sigma1(W[i-2]) + W[i-7] + sigma0(W[i-15]) + W[i-16];
    }

    // Initialize working variables
    a = state[0];
    b = state[1];
    c = state[2];
    d = state[3];
    e = state[4];
    f = state[5];
    g = state[6];
    h = state[7];

    // 80 rounds
    for (int i = 0; i < 80; i++) {
        T1 = h + SIGMA1(e) + CH(e, f, g) + K[i] + W[i];
        T2 = SIGMA0(a) + MAJ(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + T1;
        d = c;
        c = b;
        b = a;
        a = T1 + T2;
    }

    // Add compressed chunk to current hash value
    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
    state[4] += e;
    state[5] += f;
    state[6] += g;
    state[7] += h;
}

/**
 * Complete SHA-512 hash with padding
 */
void sha512_hash(const uchar *data, uint len, uchar *hash) {
    ulong state[8];
    uchar block[128];

    // Initialize hash values
    for (int i = 0; i < 8; i++) {
        state[i] = H0[i];
    }

    // Process complete blocks
    uint blocks = len / 128;
    for (uint b = 0; b < blocks; b++) {
        sha512_transform(state, data + b * 128);
    }

    // Prepare final block with padding
    uint remaining = len % 128;
    for (uint i = 0; i < remaining; i++) {
        block[i] = data[blocks * 128 + i];
    }

    // Append '1' bit (0x80)
    block[remaining] = 0x80;

    // Pad with zeros
    for (uint i = remaining + 1; i < 128; i++) {
        block[i] = 0;
    }

    // If not enough room for length, process block and create new one
    if (remaining >= 112) {
        sha512_transform(state, block);
        for (int i = 0; i < 128; i++) {
            block[i] = 0;
        }
    }

    // Append length in bits (big-endian, last 16 bytes)
    ulong bit_len = (ulong)len * 8;
    block[120] = (uchar)(bit_len >> 56);
    block[121] = (uchar)(bit_len >> 48);
    block[122] = (uchar)(bit_len >> 40);
    block[123] = (uchar)(bit_len >> 32);
    block[124] = (uchar)(bit_len >> 24);
    block[125] = (uchar)(bit_len >> 16);
    block[126] = (uchar)(bit_len >> 8);
    block[127] = (uchar)(bit_len);

    // Process final block
    sha512_transform(state, block);

    // Convert hash to bytes (big-endian)
    for (int i = 0; i < 8; i++) {
        hash[i*8 + 0] = (uchar)(state[i] >> 56);
        hash[i*8 + 1] = (uchar)(state[i] >> 48);
        hash[i*8 + 2] = (uchar)(state[i] >> 40);
        hash[i*8 + 3] = (uchar)(state[i] >> 32);
        hash[i*8 + 4] = (uchar)(state[i] >> 24);
        hash[i*8 + 5] = (uchar)(state[i] >> 16);
        hash[i*8 + 6] = (uchar)(state[i] >> 8);
        hash[i*8 + 7] = (uchar)(state[i]);
    }
}

/**
 * Check if hash meets target difficulty
 * Returns 1 if hash <= target, 0 otherwise
 */
int hash_meets_target(const uchar *hash, __global const uchar *target) {
    // Compare from most significant byte (big-endian comparison)
    for (int i = 0; i < 64; i++) {
        if (hash[i] < target[i]) return 1;
        if (hash[i] > target[i]) return 0;
    }
    return 1; // Equal is valid
}

/**
 * GPU Mining Kernel
 *
 * Each work-item tries a different nonce value
 *
 * @param version      Block version
 * @param prev_hash    Previous block hash (64 bytes)
 * @param merkle_root  Merkle root (64 bytes)
 * @param timestamp    Block timestamp
 * @param bits         Difficulty bits
 * @param nonce_start  Starting nonce for this batch
 * @param target       Target hash (64 bytes)
 * @param result       Output: found nonce (0xFFFFFFFF if not found)
 */
__kernel void mine_block(
    uint version,
    __global const uchar *prev_hash,
    __global const uchar *merkle_root,
    ulong timestamp,
    uint bits,
    uint nonce_start,
    __global const uchar *target,
    __global uint *result
) {
    uint global_id = get_global_id(0);
    uint nonce = nonce_start + global_id;

    // Serialize header with this nonce
    uchar header[148];
    serialize_header(version, prev_hash, merkle_root, timestamp, bits, nonce, header);

    // Compute SHA-512 hash
    uchar hash[64];
    sha512_hash(header, 148, hash);

    // Check if hash meets target
    if (hash_meets_target(hash, target)) {
        // Found valid nonce! Write to result using OpenCL 1.0 compatible atomic
        // Use atomic_cmpxchg to atomically update result with minimum nonce
        uint old = *result;
        while (nonce < old) {
            uint prev = atomic_cmpxchg(result, old, nonce);
            if (prev == old) break;  // Successfully updated
            old = prev;  // Try again with new value
        }
    }
}