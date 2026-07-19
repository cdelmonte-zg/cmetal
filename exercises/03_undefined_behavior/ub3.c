// ub3.c - Integer promotion: the hidden signed int
//
// Arithmetic in C never happens on types narrower than int. Operands of
// type uint8_t, uint16_t or unsigned short are first promoted -- to
// SIGNED int, because int can represent all their values (C11 6.3.1.1).
//
// So `uint16_t * uint16_t` is a signed multiplication: 0xFFFF * 0xFFFF
// = 4294836225, which is larger than INT_MAX (2147483647). That is
// signed overflow -- undefined behavior -- in code with no signed type
// in sight. Without sanitizers it usually "works" (the wrapped bits
// happen to be right); UBSan reports it for what it is.
//
// Fix: cast one operand to uint32_t BEFORE the arithmetic, so the
// promotion target is unsigned and the multiplication is well-defined.

#include <stdio.h>
#include <stdint.h>

// Returns x*x. The RESULT always fits in uint32_t (0xFFFF^2 < 2^32) --
// the problem is the type the multiplication itself happens in.
// TODO: for x > 46340 the promoted signed multiply overflows INT_MAX.
uint32_t square_u16(uint16_t x) {
    return x * x;  // BUG: both operands are promoted to signed int
}

// Returns the dot product a[0]*b[0] + ... + a[n-1]*b[n-1].
// The accumulator is uint32_t (unsigned wraparound is well-defined),
// but each product has the same promotion trap.
// TODO: make each multiplication happen in uint32_t.
uint32_t dot_u16(const uint16_t *a, const uint16_t *b, int n) {
    uint32_t sum = 0;
    for (int i = 0; i < n; i++) {
        sum += a[i] * b[i];  // BUG: promoted to signed int, may overflow
    }
    return sum;
}

#ifndef TEST
int main(void) {
    printf("square_u16(300)   = %u\n", square_u16(300));
    printf("square_u16(65535) = %u\n", square_u16(65535));

    uint16_t a[3] = {1000, 2000, 65535};
    uint16_t b[3] = {1000, 2000, 65535};
    printf("dot_u16           = %u\n", dot_u16(a, b, 3));

    return 0;
}
#else
#include "clings_test.h"

TEST(test_square_small) {
    ASSERT_EQ(square_u16(0), 0u);
    ASSERT_EQ(square_u16(1), 1u);
    ASSERT_EQ(square_u16(300), 90000u);
}

TEST(test_square_around_int_max) {
    /* 46340^2 fits in int; 46341^2 does not. */
    ASSERT_EQ(square_u16(46340), 2147395600u);
    ASSERT_EQ(square_u16(46341), 2147488281u);
}

TEST(test_square_max) {
    ASSERT_EQ(square_u16(65535), 4294836225u);
}

TEST(test_dot_small) {
    uint16_t a[3] = {1, 2, 3};
    uint16_t b[3] = {4, 5, 6};
    ASSERT_EQ(dot_u16(a, b, 3), 32u);
}

TEST(test_dot_large_products) {
    uint16_t a[2] = {65535, 65535};
    uint16_t b[2] = {65535, 65535};
    /* 2 * 4294836225 wraps mod 2^32 -- well-defined for unsigned. */
    ASSERT_EQ(dot_u16(a, b, 2), 4294705154u);
}

TEST(test_dot_empty) {
    ASSERT_EQ(dot_u16(NULL, NULL, 0), 0u);
}

int main(void) {
    RUN_TEST(test_square_small);
    RUN_TEST(test_square_around_int_max);
    RUN_TEST(test_square_max);
    RUN_TEST(test_dot_small);
    RUN_TEST(test_dot_large_products);
    RUN_TEST(test_dot_empty);
    TEST_REPORT();
}
#endif
