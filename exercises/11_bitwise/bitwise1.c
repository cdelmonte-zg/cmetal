// bitwise1.c - Bit counting and power of two
//
// Bitwise operations let you manipulate individual bits in a number.
// The & (AND), | (OR), ^ (XOR), ~ (NOT), << (left shift), and >> (right shift)
// operators work on the binary representation of integers.
//
// Fix the count_set_bits() and is_power_of_two() functions.

#include <stdio.h>
#include <limits.h>

// NOTE: this file's tests assume unsigned int is 32 bits — true on
// every platform clings targets, but an ABI fact, not a C guarantee
// (C11 only requires at least 16 value bits). The assert makes the
// assumption explicit; a good count_set_bits fix works for ANY width.
_Static_assert(UINT_MAX == 0xFFFFFFFFu,
               "bitwise1's tests assume 32-bit unsigned int");

// count_set_bits: return the number of 1-bits in n (popcount).
// BUG: The loop shifts in the wrong direction and only checks 16 bits.
int count_set_bits(unsigned int n) {
    int count = 0;
    for (int i = 0; i < 16; i++) {
        if (n & (1u << i)) {
            count++;
        }
        n <<= 1;  // BUG: shifting n left loses high bits
    }
    return count;
}

// is_power_of_two: return 1 if n is a power of 2, 0 otherwise.
// A power of 2 has exactly one bit set: n & (n - 1) == 0.
// BUG: Doesn't handle n == 0 (which is not a power of 2).
int is_power_of_two(unsigned int n) {
    return (n & (n - 1)) == 0;
}

#ifndef TEST
int main(void) {
    printf("count_set_bits(0xFF) = %d\n", count_set_bits(0xFF));
    printf("count_set_bits(0xFFFFFFFF) = %d\n", count_set_bits(0xFFFFFFFF));
    printf("is_power_of_two(64) = %d\n", is_power_of_two(64));
    printf("is_power_of_two(0) = %d\n", is_power_of_two(0));
    return 0;
}
#else
#include "clings_test.h"

TEST(test_count_zero) {
    ASSERT_EQ(count_set_bits(0), 0);
}

TEST(test_count_0xff) {
    ASSERT_EQ(count_set_bits(0xFF), 8);
}

TEST(test_count_all_ones) {
    /* 32 set bits — pinned by the _Static_assert on UINT_MAX above. */
    ASSERT_EQ(count_set_bits(UINT_MAX), 32);
}

TEST(test_power_of_two_one) {
    ASSERT_EQ(is_power_of_two(1), 1);
}

TEST(test_power_of_two_64) {
    ASSERT_EQ(is_power_of_two(64), 1);
}

TEST(test_power_of_two_zero) {
    ASSERT_EQ(is_power_of_two(0), 0);
}

TEST(test_power_of_two_six) {
    ASSERT_EQ(is_power_of_two(6), 0);
}

int main(void) {
    RUN_TEST(test_count_zero);
    RUN_TEST(test_count_0xff);
    RUN_TEST(test_count_all_ones);
    RUN_TEST(test_power_of_two_one);
    RUN_TEST(test_power_of_two_64);
    RUN_TEST(test_power_of_two_zero);
    RUN_TEST(test_power_of_two_six);
    TEST_REPORT();
}
#endif
