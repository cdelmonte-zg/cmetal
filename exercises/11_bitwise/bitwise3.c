// bitwise3.c - Bitwise tricks
//
// Many clever algorithms use bitwise tricks to avoid loops or branches.
// For example, you can round up to the next power of two by spreading
// the highest set bit downward, then adding 1.
//
// Fix the next_power_of_two(), highest_set_bit(), and swap_nibbles() functions.

#include <stdio.h>
#include <limits.h>

// The bit-smearing in next_power_of_two (n |= n >> 16 as the last
// step) and the tests assume 32-bit unsigned int — an ABI fact, not
// a C guarantee. The assert makes the assumption explicit.
_Static_assert(UINT_MAX == 0xFFFFFFFFu,
               "bitwise3 assumes 32-bit unsigned int");

// next_power_of_two: return the smallest power of 2 that is >= n.
// Uses the bit-smearing trick: spread the highest set bit to fill all
// lower bits, then add 1.
// BUG: Doesn't handle the case when n is already a power of 2.
// When n is already a power of 2, we should return n itself, not 2*n.
unsigned int next_power_of_two(unsigned int n) {
    if (n == 0) {
        return 1;
    }
    // Missing: n-- to handle exact powers of two
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    return n + 1;
}

// highest_set_bit: return the 0-indexed position of the highest set bit.
// Returns -1 if n == 0.
// BUG: Off-by-one error -- starts pos at 0 but increments before checking,
// so the result is one too high.
int highest_set_bit(unsigned int n) {
    if (n == 0) {
        return -1;
    }
    int pos = 0;
    while (n >>= 1) {
        pos++;
    }
    return pos + 1;  // BUG: off by one
}

// swap_nibbles: swap the low nibble (bits 3-0) and high nibble (bits 7-4)
// of the lowest byte. For example, 0xAB becomes 0xBA.
// Only operates on the low 8 bits; upper bits should be zero.
unsigned int swap_nibbles(unsigned int byte) {
    unsigned int low = byte & 0x0F;
    unsigned int high = (byte >> 4) & 0x0F;
    return (low << 4) | high;
}

#ifndef TEST
int main(void) {
    printf("next_power_of_two(5) = %u\n", next_power_of_two(5));
    printf("next_power_of_two(8) = %u\n", next_power_of_two(8));
    printf("next_power_of_two(1) = %u\n", next_power_of_two(1));
    printf("highest_set_bit(1) = %d\n", highest_set_bit(1));
    printf("highest_set_bit(128) = %d\n", highest_set_bit(128));
    printf("swap_nibbles(0xAB) = 0x%X\n", swap_nibbles(0xAB));
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_next_pow2_five) {
    ASSERT_EQ(next_power_of_two(5), 8u);
}

TEST(test_next_pow2_eight) {
    ASSERT_EQ(next_power_of_two(8), 8u);
}

TEST(test_next_pow2_one) {
    ASSERT_EQ(next_power_of_two(1), 1u);
}

TEST(test_next_pow2_zero) {
    ASSERT_EQ(next_power_of_two(0), 1u);
}

TEST(test_highest_bit_one) {
    ASSERT_EQ(highest_set_bit(1), 0);
}

TEST(test_highest_bit_128) {
    ASSERT_EQ(highest_set_bit(128), 7);
}

TEST(test_highest_bit_zero) {
    ASSERT_EQ(highest_set_bit(0), -1);
}

TEST(test_highest_bit_255) {
    ASSERT_EQ(highest_set_bit(255), 7);
}

TEST(test_swap_nibbles_ab) {
    ASSERT_EQ(swap_nibbles(0xAB), 0xBAu);
}

TEST(test_swap_nibbles_12) {
    ASSERT_EQ(swap_nibbles(0x12), 0x21u);
}

TEST(test_swap_nibbles_f0) {
    ASSERT_EQ(swap_nibbles(0xF0), 0x0Fu);
}

int main(void) {
    RUN_TEST(test_next_pow2_five);
    RUN_TEST(test_next_pow2_eight);
    RUN_TEST(test_next_pow2_one);
    RUN_TEST(test_next_pow2_zero);
    RUN_TEST(test_highest_bit_one);
    RUN_TEST(test_highest_bit_128);
    RUN_TEST(test_highest_bit_zero);
    RUN_TEST(test_highest_bit_255);
    RUN_TEST(test_swap_nibbles_ab);
    RUN_TEST(test_swap_nibbles_12);
    RUN_TEST(test_swap_nibbles_f0);
    TEST_REPORT();
}
#endif
