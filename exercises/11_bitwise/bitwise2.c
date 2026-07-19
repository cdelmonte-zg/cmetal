// bitwise2.c - Bit field extraction and packing
//
// Colors on screen are often stored as a single 32-bit integer in
// 0x00RRGGBB format: bits 23-16 hold red, 15-8 hold green, 7-0 hold blue.
// Packing and unpacking these fields requires shifting and masking.
//
// Fix the pack_rgb(), unpack_rgb(), and extract_bits() functions.

#include <stdio.h>
#include <stdint.h>

// pack_rgb: pack r, g, b into a single uint32_t in 0x00RRGGBB format.
// BUG: Red is shifted by 8 instead of 16.
uint32_t pack_rgb(uint8_t r, uint8_t g, uint8_t b) {
    return ((uint32_t)r << 8) | ((uint32_t)g << 8) | (uint32_t)b;
}

// unpack_rgb: extract the red, green, and blue components from a packed color.
void unpack_rgb(uint32_t color, uint8_t *r, uint8_t *g, uint8_t *b) {
    *r = (color >> 16) & 0xFF;
    *g = (color >> 8) & 0xFF;
    *b = color & 0xFF;
}

// extract_bits: extract `count` bits starting at bit position `start`.
// Precondition: 0 < count < 32. With count == 32 the mask expression
// would shift by the full width of the type — undefined behavior in C
// (C11 6.5.7p3) — so full-word extraction is outside this contract.
// For example, extract_bits(0xABCD, 4, 8) should return 0xBC.
// BUG: The mask calculation is wrong.
uint32_t extract_bits(uint32_t value, int start, int count) {
    uint32_t mask = (1u << count);  // BUG: should be (1u << count) - 1
    return (value >> start) & mask;
}

#ifndef TEST
int main(void) {
    uint32_t color = pack_rgb(0xFF, 0x00, 0x80);
    printf("Packed: 0x%08X\n", color);

    uint8_t r, g, b;
    unpack_rgb(color, &r, &g, &b);
    printf("Unpacked: R=0x%02X G=0x%02X B=0x%02X\n", r, g, b);

    printf("extract_bits(0xABCD, 4, 8) = 0x%X\n", extract_bits(0xABCD, 4, 8));
    return 0;
}
#else
#include "clings_test.h"

TEST(test_pack_rgb) {
    ASSERT_EQ(pack_rgb(0xFF, 0x00, 0x80), (uint32_t)0x00FF0080);
}

TEST(test_pack_rgb_white) {
    ASSERT_EQ(pack_rgb(0xFF, 0xFF, 0xFF), (uint32_t)0x00FFFFFF);
}

TEST(test_pack_rgb_black) {
    ASSERT_EQ(pack_rgb(0x00, 0x00, 0x00), (uint32_t)0x00000000);
}

TEST(test_unpack_roundtrip) {
    uint32_t color = pack_rgb(0xAA, 0xBB, 0xCC);
    uint8_t r, g, b;
    unpack_rgb(color, &r, &g, &b);
    ASSERT_EQ(r, 0xAA);
    ASSERT_EQ(g, 0xBB);
    ASSERT_EQ(b, 0xCC);
}

TEST(test_unpack_known) {
    uint8_t r, g, b;
    unpack_rgb(0x00FF0080, &r, &g, &b);
    ASSERT_EQ(r, 0xFF);
    ASSERT_EQ(g, 0x00);
    ASSERT_EQ(b, 0x80);
}

TEST(test_extract_bits_middle) {
    ASSERT_EQ(extract_bits(0xABCD, 4, 8), (uint32_t)0xBC);
}

TEST(test_extract_bits_low) {
    ASSERT_EQ(extract_bits(0xFF, 0, 4), (uint32_t)0x0F);
}

TEST(test_extract_bits_high) {
    ASSERT_EQ(extract_bits(0xF000, 12, 4), (uint32_t)0x0F);
}

int main(void) {
    RUN_TEST(test_pack_rgb);
    RUN_TEST(test_pack_rgb_white);
    RUN_TEST(test_pack_rgb_black);
    RUN_TEST(test_unpack_roundtrip);
    RUN_TEST(test_unpack_known);
    RUN_TEST(test_extract_bits_middle);
    RUN_TEST(test_extract_bits_low);
    RUN_TEST(test_extract_bits_high);
    TEST_REPORT();
}
#endif
