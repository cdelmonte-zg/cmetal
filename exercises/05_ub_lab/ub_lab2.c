// ub_lab2.c - Strict aliasing violation
//
// C's "strict aliasing rule" says you may not access an object through a
// pointer of an incompatible type. Casting a float* to uint32_t* and
// dereferencing it violates this rule and is UNDEFINED BEHAVIOR.
//
// This "type-punning" trick is extremely common in old code (especially
// for the famous fast inverse square root hack), but it is not legal C.
//
// Fix: use memcpy() to copy the bytes between types. The compiler will
// optimize the memcpy into a register move -- zero overhead.

#include <stdio.h>
#include <stdint.h>
#include <string.h>

// Returns the raw IEEE 754 bits of a float as a uint32_t.
uint32_t float_to_bits(float f) {
    // BUG: strict aliasing violation!
    // Accessing a float object through a uint32_t pointer is UB.
    // TODO: Use memcpy() instead of pointer casting.
    uint32_t *p = (uint32_t *)&f;
    return *p;
}

// Reconstructs a float from its raw IEEE 754 bits.
float bits_to_float(uint32_t bits) {
    // BUG: same strict aliasing violation in the other direction.
    // TODO: Use memcpy() instead of pointer casting.
    float *p = (float *)&bits;
    return *p;
}

// Returns non-zero if the float is negative (checks the sign bit).
int float_is_negative(float f) {
    uint32_t bits = float_to_bits(f);
    return (bits >> 31) & 1;
}

#ifndef TEST
int main(void) {
    float val = -3.14f;
    uint32_t bits = float_to_bits(val);

    printf("Float: %f\n", (double)val);
    printf("Bits:  0x%08X\n", bits);
    printf("Sign:  %s\n", float_is_negative(val) ? "negative" : "positive");

    float reconstructed = bits_to_float(bits);
    printf("Reconstructed: %f\n", (double)reconstructed);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_zero_bits) {
    uint32_t bits = float_to_bits(0.0f);
    ASSERT_EQ(bits, 0u);
}

TEST(test_one_bits) {
    /* IEEE 754: 1.0f = 0x3F800000 */
    uint32_t bits = float_to_bits(1.0f);
    ASSERT_EQ(bits, 0x3F800000u);
}

TEST(test_roundtrip) {
    float original = 42.5f;
    uint32_t bits = float_to_bits(original);
    float result = bits_to_float(bits);
    ASSERT(original == result);
}

TEST(test_negative_roundtrip) {
    float original = -123.456f;
    uint32_t bits = float_to_bits(original);
    float result = bits_to_float(bits);
    ASSERT(original == result);
}

TEST(test_sign_positive) {
    ASSERT(!float_is_negative(1.0f));
    ASSERT(!float_is_negative(0.0f));
}

TEST(test_sign_negative) {
    ASSERT(float_is_negative(-1.0f));
    ASSERT(float_is_negative(-0.001f));
}

int main(void) {
    RUN_TEST(test_zero_bits);
    RUN_TEST(test_one_bits);
    RUN_TEST(test_roundtrip);
    RUN_TEST(test_negative_roundtrip);
    RUN_TEST(test_sign_positive);
    RUN_TEST(test_sign_negative);
    TEST_REPORT();
}
#endif
