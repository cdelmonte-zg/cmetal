// encodings2.c - Varints: LEB128 with a defensive decoder
//
// Variable-length integers spend one byte per 7 bits of payload: the
// low 7 bits carry data, the high bit says "another byte follows".
// Small numbers — the common case in bytecode, lengths, deltas — cost
// one byte instead of four.
//
// Encoding is the easy half. DECODING is parsing untrusted input, and
// this decoder trusts it completely:
//   1. it never checks `len`, so a truncated buffer is read past its
//      end (the demo feeds it an exactly-sized heap buffer — ASan has
//      opinions);
//   2. it never bounds the shift: a fifth continuation byte shifts by
//      35 — undefined behavior (C11 6.5.7p3) — and values that don't
//      fit in 32 bits are silently accepted with their top bits gone.
//
// TODO: make varint_decode reject bad input instead of trusting it.
// Contract: returns bytes consumed (1..5) on success; 0 on error
// (truncated input, a continuation past the 5th byte, or a value that
// doesn't fit in 32 bits) — and on error *out is left untouched.

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>

// Encodes value as LEB128 into out. Returns bytes written (1..5).
size_t varint_encode(uint32_t value, uint8_t out[5]) {
    size_t i = 0;
    while (value >= 0x80) {
        out[i++] = (uint8_t)(value | 0x80);
        value >>= 7;
    }
    out[i++] = (uint8_t)value;
    return i;
}

// Decodes a LEB128 value from in[0..len). Returns bytes consumed
// (1..5), or 0 on error (*out untouched in that case).
size_t varint_decode(const uint8_t *in, size_t len, uint32_t *out) {
    uint32_t result = 0;
    unsigned shift = 0;
    size_t i = 0;
    // BUG: `len` is ignored (out-of-bounds read on truncated input),
    // the shift is unbounded (UB past the 5th byte), and 32-bit
    // overflow is silently accepted.
    (void)len;
    for (;;) {
        uint8_t byte = in[i];
        result |= (uint32_t)(byte & 0x7F) << shift;
        i++;
        if (!(byte & 0x80)) {
            break;
        }
        shift += 7;
    }
    *out = result;
    return i;
}

#ifndef TEST
int main(void) {
    uint8_t buf[5];
    uint32_t v = 0;

    size_t n = varint_encode(300, buf);
    size_t m = varint_decode(buf, n, &v);
    printf("300 -> %zu byte(s) -> %u (consumed %zu)\n", n, v, m);

    // Parsing untrusted input: a truncated varint must be REJECTED,
    // not read past the end of the buffer. The buffer is heap-allocated
    // at its exact size so the sanitizer can see any overread.
    uint8_t *truncated = malloc(2);
    if (!truncated) return 1;
    truncated[0] = 0x80;
    truncated[1] = 0x80;  // continuation bit set, but the data ends here
    if (varint_decode(truncated, 2, &v) == 0) {
        printf("truncated input: rejected\n");
    } else {
        printf("truncated input: ACCEPTED (bad!)\n");
    }
    free(truncated);

    return 0;
}
#else
#include "clings_test.h"

TEST(test_encode_known_bytes) {
    uint8_t buf[5];
    ASSERT_EQ(varint_encode(127, buf), 1u);
    ASSERT_EQ(buf[0], 0x7F);
    ASSERT_EQ(varint_encode(128, buf), 2u);
    ASSERT_EQ(buf[0], 0x80);
    ASSERT_EQ(buf[1], 0x01);
}

TEST(test_roundtrip) {
    const uint32_t cases[] = {0, 1, 127, 128, 300, 16384, 0xDEADBEEF, UINT32_MAX};
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        uint8_t buf[5];
        uint32_t v = 0;
        size_t n = varint_encode(cases[i], buf);
        ASSERT_EQ(varint_decode(buf, n, &v), n);
        ASSERT_EQ(v, cases[i]);
    }
}

TEST(test_max_value_is_five_bytes) {
    uint8_t buf[5];
    uint32_t v = 0;
    ASSERT_EQ(varint_encode(UINT32_MAX, buf), 5u);
    ASSERT_EQ(buf[4], 0x0F);
    ASSERT_EQ(varint_decode(buf, 5, &v), 5u);
    ASSERT_EQ(v, UINT32_MAX);
}

TEST(test_truncated_is_rejected) {
    // Continuation bit set on the last available byte: the value is
    // incomplete. The decode must fail and leave *out untouched.
    const uint8_t buf[8] = {0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
    uint32_t v = 99;
    ASSERT_EQ(varint_decode(buf, 2, &v), 0u);
    ASSERT_EQ(v, 99u);
}

TEST(test_overflow_is_rejected) {
    // 5 bytes whose 5th carries more than the 4 bits that still fit in
    // a uint32_t: accepting it would silently drop the top bits.
    const uint8_t buf[5] = {0xFF, 0xFF, 0xFF, 0xFF, 0x7F};
    uint32_t v = 99;
    ASSERT_EQ(varint_decode(buf, 5, &v), 0u);
    ASSERT_EQ(v, 99u);
}

TEST(test_six_continuations_rejected) {
    // A uint32_t varint can never need more than 5 bytes: byte 6 with
    // the data still "continuing" is malformed input, not a big number.
    const uint8_t buf[6] = {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01};
    uint32_t v = 99;
    ASSERT_EQ(varint_decode(buf, 6, &v), 0u);
    ASSERT_EQ(v, 99u);
}

int main(void) {
    RUN_TEST(test_encode_known_bytes);
    RUN_TEST(test_roundtrip);
    RUN_TEST(test_max_value_is_five_bytes);
    RUN_TEST(test_truncated_is_rejected);
    RUN_TEST(test_overflow_is_rejected);
    RUN_TEST(test_six_continuations_rejected);
    TEST_REPORT();
}
#endif
