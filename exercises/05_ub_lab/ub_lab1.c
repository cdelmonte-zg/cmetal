// ub_lab1.c - Shifting into the sign bit
//
// In C11, left-shifting a 1 into (or past) the sign bit of a signed integer
// is UNDEFINED BEHAVIOR. The expression `1 << 31` does exactly this on
// systems where int is 32 bits. It "works" on most compilers but UBSan
// will catch it.
//
// Fix the function to use unsigned arithmetic for the shift.

#include <stdio.h>

// Sets bit n in *flags (0-indexed from LSB)
void set_bit(unsigned int *flags, int n) {
    // BUG: `1 << 31` is UB when int is 32 bits because 1 is signed.
    // The shift moves a 1 into the sign bit position.
    // TODO: Make the left operand unsigned so the shift is well-defined.
    *flags |= (1 << n);
}

// Returns non-zero if bit n is set in flags
int check_bit(unsigned int flags, int n) {
    // Same bug here: shifting signed 1 into the sign bit.
    return (flags & (1 << n)) != 0;
}

// Clears bit n in *flags
void clear_bit(unsigned int *flags, int n) {
    // And here too.
    *flags &= ~(1 << n);
}

#ifndef TEST
int main(void) {
    unsigned int flags = 0;

    set_bit(&flags, 0);
    set_bit(&flags, 15);
    set_bit(&flags, 31);

    printf("Bit  0: %s\n", check_bit(flags, 0)  ? "set" : "clear");
    printf("Bit 15: %s\n", check_bit(flags, 15) ? "set" : "clear");
    printf("Bit 31: %s\n", check_bit(flags, 31) ? "set" : "clear");
    printf("Bit  7: %s\n", check_bit(flags, 7)  ? "set" : "clear");

    clear_bit(&flags, 15);
    printf("Bit 15 after clear: %s\n", check_bit(flags, 15) ? "set" : "clear");

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_set_low_bit) {
    unsigned int flags = 0;
    set_bit(&flags, 0);
    ASSERT(check_bit(flags, 0));
}

TEST(test_set_middle_bit) {
    unsigned int flags = 0;
    set_bit(&flags, 15);
    ASSERT(check_bit(flags, 15));
    ASSERT(!check_bit(flags, 14));
}

TEST(test_set_high_bit) {
    unsigned int flags = 0;
    set_bit(&flags, 31);
    ASSERT(check_bit(flags, 31));
}

TEST(test_clear_bit) {
    unsigned int flags = 0;
    set_bit(&flags, 10);
    ASSERT(check_bit(flags, 10));
    clear_bit(&flags, 10);
    ASSERT(!check_bit(flags, 10));
}

TEST(test_multiple_bits) {
    unsigned int flags = 0;
    set_bit(&flags, 0);
    set_bit(&flags, 15);
    set_bit(&flags, 31);
    ASSERT(check_bit(flags, 0));
    ASSERT(check_bit(flags, 15));
    ASSERT(check_bit(flags, 31));
    ASSERT(!check_bit(flags, 7));
}

TEST(test_clear_preserves_others) {
    unsigned int flags = 0;
    set_bit(&flags, 5);
    set_bit(&flags, 10);
    clear_bit(&flags, 5);
    ASSERT(!check_bit(flags, 5));
    ASSERT(check_bit(flags, 10));
}

int main(void) {
    RUN_TEST(test_set_low_bit);
    RUN_TEST(test_set_middle_bit);
    RUN_TEST(test_set_high_bit);
    RUN_TEST(test_clear_bit);
    RUN_TEST(test_multiple_bits);
    RUN_TEST(test_clear_preserves_others);
    TEST_REPORT();
}
#endif
