// encodings3.c - Bit packing
//
// Columnar formats don't spend a byte on a value that fits in 5 bits:
// they pack values back to back at a fixed bit width, LSB-first, and
// flush full bytes as they form. 8 values at width 5 = 5 bytes, not 8.
//
// Two bugs stand between this packer and that promise:
//   1. the width mask is built as (1u << width) - 1 — undefined
//      behavior at width == 32, a shift by the full type width
//      (C11 6.5.7p3). bitwise2 documented this exact precondition;
//      here the contract ALLOWS width 32, so the mask must be built
//      differently (hint: a wider accumulator already exists).
//   2. when the total bit count is not a multiple of 8, the final
//      partial byte is left in the accumulator and never written.
//
// TODO: fix both functions. Contract: 1 <= width <= 32; every value
// fits in `width` bits; the byte count is (count*width + 7) / 8, and
// pack/unpack must round-trip for every legal width.

#include <stdio.h>
#include <stdint.h>

// Packs count values of `width` bits each into out, LSB-first.
// Returns the number of bytes written: (count*width + 7) / 8.
size_t pack_bits(const uint32_t *values, size_t count, unsigned width, uint8_t *out) {
    // BUG: undefined behavior when width == 32.
    uint32_t mask = (1u << width) - 1;
    uint64_t acc = 0;
    unsigned nbits = 0;
    size_t n = 0;
    for (size_t i = 0; i < count; i++) {
        acc |= (uint64_t)(values[i] & mask) << nbits;
        nbits += width;
        while (nbits >= 8) {
            out[n++] = (uint8_t)(acc & 0xFF);
            acc >>= 8;
            nbits -= 8;
        }
    }
    // BUG: a final partial byte is still sitting in acc.
    return n;
}

// Unpacks count values of `width` bits each from in, LSB-first.
// Returns the number of bytes consumed: (count*width + 7) / 8.
size_t unpack_bits(const uint8_t *in, size_t count, unsigned width, uint32_t *values) {
    // BUG: same undefined mask at width == 32.
    uint32_t mask = (1u << width) - 1;
    uint64_t acc = 0;
    unsigned nbits = 0;
    size_t n = 0;
    for (size_t i = 0; i < count; i++) {
        while (nbits < width) {
            acc |= (uint64_t)in[n++] << nbits;
            nbits += 8;
        }
        values[i] = (uint32_t)(acc & mask);
        acc >>= width;
        nbits -= width;
    }
    return n;
}

#ifndef TEST
int main(void) {
    // Width 5: 8 values in 5 bytes.
    const uint32_t small[8] = {3, 7, 31, 0, 15, 1, 22, 9};
    uint8_t buf[64];
    uint32_t back[8] = {0};
    size_t n = pack_bits(small, 8, 5, buf);
    printf("8 values @ 5 bits -> %zu bytes\n", n);
    unpack_bits(buf, 8, 5, back);
    printf("roundtrip: %u %u %u ... %u\n", back[0], back[1], back[2], back[7]);

    // Width 32 is a legal width: full 32-bit values pass through.
    const uint32_t wide[2] = {0xDEADBEEFu, UINT32_MAX};
    uint32_t wide_back[2] = {0};
    n = pack_bits(wide, 2, 32, buf);
    unpack_bits(buf, 2, 32, wide_back);
    printf("width 32: %zu bytes, %08X %08X\n", n, wide_back[0], wide_back[1]);

    return 0;
}
#else
#include "clings_test.h"

TEST(test_width1_is_a_bitmap) {
    const uint32_t bits[8] = {1, 0, 1, 1, 0, 1, 0, 0};
    uint8_t buf[8] = {0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA};
    ASSERT_EQ(pack_bits(bits, 8, 1, buf), 1u);
    ASSERT_EQ(buf[0], 0x2D); /* 0b00101101, LSB-first */
}

TEST(test_width5_known_bytes) {
    const uint32_t vals[8] = {3, 7, 31, 0, 15, 1, 22, 9};
    uint8_t buf[8];
    ASSERT_EQ(pack_bits(vals, 8, 5, buf), 5u);
    uint32_t back[8] = {0};
    ASSERT_EQ(unpack_bits(buf, 8, 5, back), 5u);
    for (int i = 0; i < 8; i++) {
        ASSERT_EQ(back[i], vals[i]);
    }
}

TEST(test_partial_final_byte_is_flushed) {
    // 3 values @ 3 bits = 9 bits = 2 bytes; the second byte holds a
    // single data bit and must still be written.
    const uint32_t vals[3] = {5, 2, 7};
    uint8_t buf[4] = {0xEE, 0xEE, 0xEE, 0xEE};
    ASSERT_EQ(pack_bits(vals, 3, 3, buf), 2u);
    /* 5=101, 2=010, 7=111 LSB-first: bits 101 010 111 -> 0b11010101, 0b1 */
    ASSERT_EQ(buf[0], 0xD5);
    ASSERT_EQ(buf[1], 0x01);
    uint32_t back[3] = {0};
    ASSERT_EQ(unpack_bits(buf, 3, 3, back), 2u);
    ASSERT_EQ(back[0], 5u);
    ASSERT_EQ(back[1], 2u);
    ASSERT_EQ(back[2], 7u);
}

TEST(test_width32_full_values) {
    const uint32_t vals[3] = {0, 0xDEADBEEFu, UINT32_MAX};
    uint8_t buf[12];
    ASSERT_EQ(pack_bits(vals, 3, 32, buf), 12u);
    uint32_t back[3] = {1, 1, 1};
    ASSERT_EQ(unpack_bits(buf, 3, 32, back), 12u);
    ASSERT_EQ(back[0], 0u);
    ASSERT_EQ(back[1], 0xDEADBEEFu);
    ASSERT_EQ(back[2], UINT32_MAX);
}

TEST(test_roundtrip_every_width) {
    uint8_t buf[32];
    for (unsigned width = 1; width <= 32; width++) {
        /* the largest value that fits, plus a couple of small ones */
        uint32_t max = width == 32 ? UINT32_MAX : (1u << width) - 1u;
        const uint32_t vals[3] = {max, 0, max / 2};
        uint32_t back[3] = {7, 7, 7};
        size_t expected = (3 * width + 7) / 8;
        ASSERT_EQ(pack_bits(vals, 3, width, buf), expected);
        ASSERT_EQ(unpack_bits(buf, 3, width, back), expected);
        ASSERT_EQ(back[0], max);
        ASSERT_EQ(back[1], 0u);
        ASSERT_EQ(back[2], max / 2);
    }
}

int main(void) {
    RUN_TEST(test_width1_is_a_bitmap);
    RUN_TEST(test_width5_known_bytes);
    RUN_TEST(test_partial_final_byte_is_flushed);
    RUN_TEST(test_width32_full_values);
    RUN_TEST(test_roundtrip_every_width);
    TEST_REPORT();
}
#endif
