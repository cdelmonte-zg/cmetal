// capstone1.c - Writing a binary format: the layout IS the contract
//
// The capstone arc takes a bytecode chunk (topic 18) across a file
// boundary: write it, validate it, load it back. This first exercise
// is the writer. The on-disk layout, every multi-byte field
// little-endian regardless of host (topic 12):
//
//   offset  size  field
//   0       4     magic "CMBC"
//   4       2     version (currently 1)
//   6       2     const_count        <- number of CONSTANTS
//   8       8*n   constants, IEEE-754 bit patterns (topic 17's pun)
//   8+8*n   4     code_len
//   ...     m     code bytes
//
// Two bugs:
//   1. the capacity check computes the needed size WITHOUT the code
//      bytes — then writes the code anyway, past the capacity the
//      caller granted (the demo sizes a heap buffer exactly and ASan
//      names the overrun);
//   2. const_count is written as a BYTE count (n * 8) instead of an
//      element count. Every length field must say WHAT it counts;
//      a reader honoring the contract will walk off into the
//      constants.
//
// Contract: chunk_write refuses (returns -1) when cap cannot hold
// the whole serialization, otherwise writes the layout above and
// returns the number of bytes written. Counts that cannot fit their
// fields are refused (given correct).
//
// (Editorial note: every save file, wire message and cache entry is
// written by code shaped exactly like this. The reader that must
// survive this format is the next exercise.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>

typedef struct {
    const double *consts;
    size_t const_count;
    const uint8_t *code;
    size_t code_len;
} Chunk;

static inline void write_u16_le(uint8_t *p, uint16_t v) {
    p[0] = (uint8_t)(v & 0xff);
    p[1] = (uint8_t)(v >> 8);
}

static inline void write_u32_le(uint8_t *p, uint32_t v) {
    p[0] = (uint8_t)(v & 0xff);
    p[1] = (uint8_t)((v >> 8) & 0xff);
    p[2] = (uint8_t)((v >> 16) & 0xff);
    p[3] = (uint8_t)((v >> 24) & 0xff);
}

// The only legal pun: memcpy between same-sized objects (topic 17).
static inline uint64_t double_to_bits(double d) {
    uint64_t b;
    memcpy(&b, &d, sizeof b);
    return b;
}

static inline void write_u64_le(uint8_t *p, uint64_t v) {
    for (int i = 0; i < 8; i++) {
        p[i] = (uint8_t)(v >> (8 * i));
    }
}

#define CHUNK_MAGIC "CMBC"
#define CHUNK_VERSION 1

// Serializes c into buf. Returns the number of bytes written, or -1
// if cap is too small or a count does not fit its field.
long chunk_write(const Chunk *c, uint8_t *buf, size_t cap) {
    if (c->const_count > UINT16_MAX || c->code_len > UINT32_MAX) {
        return -1;
    }
    // BUG: 'needed' forgets the code bytes — the check passes with a
    // buffer that cannot hold the code, and the memcpy below writes
    // past the capacity the caller granted.
    size_t needed = 8 + c->const_count * 8 + 4;
    if (cap < needed) {
        return -1;
    }

    memcpy(buf, CHUNK_MAGIC, 4);
    write_u16_le(buf + 4, CHUNK_VERSION);
    // BUG: the field is the number of CONSTANTS, not of bytes.
    write_u16_le(buf + 6, (uint16_t)(c->const_count * 8));

    size_t off = 8;
    for (size_t i = 0; i < c->const_count; i++) {
        write_u64_le(buf + off, double_to_bits(c->consts[i]));
        off += 8;
    }
    write_u32_le(buf + off, (uint32_t)c->code_len);
    off += 4;
    memcpy(buf + off, c->code, c->code_len);
    off += c->code_len;
    return (long)off;
}

#ifndef TEST
#include <stdlib.h>

int main(void) {
    static const double consts[] = { 1.5 };
    static const uint8_t code[] = { 1, 0, 0 };
    Chunk c = { consts, 1, code, sizeof(code) };

    // A heap buffer sized to everything EXCEPT the code bytes: the
    // broken capacity check accepts it, and the code memcpy runs past
    // the end — AddressSanitizer names the write.
    size_t cap = 8 + 1 * 8 + 4;
    uint8_t *buf = malloc(cap);
    if (!buf) {
        return 1;
    }
    long written = chunk_write(&c, buf, cap);
    printf("write into a code-less buffer: %ld (expected -1)\n", written);
    free(buf);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_layout_is_exact) {
    // Golden bytes: the layout is the contract, byte for byte.
    static const double consts[] = { 1.5 }; /* bits 0x3FF8000000000000 */
    static const uint8_t code[] = { 1, 0, 0 };
    Chunk c = { consts, 1, code, sizeof(code) };
    uint8_t buf[64];
    long written = chunk_write(&c, buf, sizeof(buf));
    ASSERT_EQ(written, 23L); /* 8 + 8 + 4 + 3 */
    static const uint8_t expected[] = {
        'C', 'M', 'B', 'C',                             /* magic */
        0x01, 0x00,                                     /* version 1 */
        0x01, 0x00,                                     /* ONE constant */
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F, /* 1.5, LE */
        0x03, 0x00, 0x00, 0x00,                         /* code_len 3 */
        1, 0, 0                                         /* code */
    };
    ASSERT_EQ(memcmp(buf, expected, sizeof(expected)), 0);
}

TEST(test_exact_capacity_is_accepted) {
    static const double consts[] = { 2.0, 3.0 };
    static const uint8_t code[] = { 1, 0, 1, 1, 2, 0 };
    Chunk c = { consts, 2, code, sizeof(code) };
    uint8_t buf[64];
    size_t needed = 8 + 2 * 8 + 4 + sizeof(code);
    ASSERT_EQ(chunk_write(&c, buf, needed), (long)needed);
}

TEST(test_capacity_check_covers_the_code) {
    // One byte short of the full serialization: must refuse. The real
    // buffer is big, so a wrong ACCEPT is observable without any
    // out-of-bounds write here — the demo shows the memory side.
    static const double consts[] = { 2.0, 3.0 };
    static const uint8_t code[] = { 1, 0, 1, 1, 2, 0 };
    Chunk c = { consts, 2, code, sizeof(code) };
    uint8_t buf[64];
    size_t needed = 8 + 2 * 8 + 4 + sizeof(code);
    ASSERT_EQ(chunk_write(&c, buf, needed - 1), -1L);
    /* a buffer that only fits the header + consts must refuse too */
    ASSERT_EQ(chunk_write(&c, buf, 8 + 2 * 8 + 4), -1L);
}

TEST(test_oversized_counts_are_refused) {
    static const uint8_t code[] = { 0 };
    Chunk c = { NULL, (size_t)UINT16_MAX + 1, code, 1 };
    uint8_t buf[64];
    ASSERT_EQ(chunk_write(&c, buf, sizeof(buf)), -1L);
}

int main(void) {
    RUN_TEST(test_layout_is_exact);
    RUN_TEST(test_exact_capacity_is_accepted);
    RUN_TEST(test_capacity_check_covers_the_code);
    RUN_TEST(test_oversized_counts_are_refused);
    TEST_REPORT();
}
#endif
