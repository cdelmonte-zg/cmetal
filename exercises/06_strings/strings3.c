// strings3.c - String to integer (manual atoi)
//
// Implement my_strtoi() that safely converts a string to an int.
// It handles leading whitespace, an optional +/- sign, and decimal digits.
// It stops at the first non-digit character after the number.
//
// Returns:
//    0  on success (result stored in *result)
//   -1  on invalid input (no digits found)
//   -2  on overflow (value exceeds INT_MIN..INT_MAX)
//
// There are TWO bugs:
// 1. Negative numbers are not handled correctly (sign is never applied)
// 2. Overflow is not detected (large values silently wrap around)
// Find and fix both!

#include <stdio.h>
#include <limits.h>

/* The overflow test STRINGS ("2147483648", ...) assume 32-bit int; the
 * assertions themselves use INT_MAX/INT_MIN. C11 only guarantees int
 * can hold ±32767 — the assert makes the real assumption explicit. */
_Static_assert(INT_MAX == 2147483647,
               "strings3's overflow test strings assume 32-bit int");

int my_strtoi(const char *s, int *result) {
    if (s == NULL) {
        return -1;
    }

    // Skip leading whitespace
    while (*s == ' ' || *s == '\t' || *s == '\n') {
        s++;
    }

    // Handle optional sign
    // BUG #1: The sign is parsed but never used in the final result.
    if (*s == '+' || *s == '-') {
        s++;
    }

    // Must have at least one digit
    if (*s < '0' || *s > '9') {
        return -1;
    }

    // BUG #2: Accumulates in int with no overflow check.
    // Large values like "2147483648" will silently overflow.
    int value = 0;
    while (*s >= '0' && *s <= '9') {
        int digit = *s - '0';
        value = value * 10 + digit;
        s++;
    }

    *result = value;
    return 0;
}

#ifndef TEST
int main(void) {
    int val;
    const char *tests[] = {"42", "-7", "  +123", "abc", "", "2147483648", NULL};

    for (int i = 0; tests[i] != NULL; i++) {
        int rc = my_strtoi(tests[i], &val);
        if (rc == 0) {
            printf("\"%s\" -> %d\n", tests[i], val);
        } else {
            printf("\"%s\" -> error %d\n", tests[i], rc);
        }
    }

    return 0;
}
#else
#include "clings_test.h"

TEST(test_simple_positive) {
    int val;
    ASSERT_EQ(my_strtoi("42", &val), 0);
    ASSERT_EQ(val, 42);
}

TEST(test_negative) {
    int val;
    ASSERT_EQ(my_strtoi("-7", &val), 0);
    ASSERT_EQ(val, -7);
}

TEST(test_leading_whitespace_and_plus) {
    int val;
    ASSERT_EQ(my_strtoi("  +123", &val), 0);
    ASSERT_EQ(val, 123);
}

TEST(test_zero) {
    int val;
    ASSERT_EQ(my_strtoi("0", &val), 0);
    ASSERT_EQ(val, 0);
}

TEST(test_invalid_no_digits) {
    int val;
    ASSERT_EQ(my_strtoi("abc", &val), -1);
}

TEST(test_invalid_empty) {
    int val;
    ASSERT_EQ(my_strtoi("", &val), -1);
}

TEST(test_invalid_null) {
    int val;
    ASSERT_EQ(my_strtoi(NULL, &val), -1);
}

TEST(test_invalid_sign_only) {
    int val;
    ASSERT_EQ(my_strtoi("-", &val), -1);
}

TEST(test_stops_at_non_digit) {
    int val;
    ASSERT_EQ(my_strtoi("99bottles", &val), 0);
    ASSERT_EQ(val, 99);
}

TEST(test_int_max) {
    int val;
    ASSERT_EQ(my_strtoi("2147483647", &val), 0);
    ASSERT_EQ(val, INT_MAX);
}

TEST(test_int_min) {
    int val;
    ASSERT_EQ(my_strtoi("-2147483648", &val), 0);
    ASSERT_EQ(val, INT_MIN);
}

TEST(test_overflow_positive) {
    int val;
    ASSERT_EQ(my_strtoi("2147483648", &val), -2);
}

TEST(test_overflow_negative) {
    int val;
    ASSERT_EQ(my_strtoi("-2147483649", &val), -2);
}

TEST(test_overflow_large) {
    int val;
    ASSERT_EQ(my_strtoi("99999999999", &val), -2);
}

int main(void) {
    RUN_TEST(test_simple_positive);
    RUN_TEST(test_negative);
    RUN_TEST(test_leading_whitespace_and_plus);
    RUN_TEST(test_zero);
    RUN_TEST(test_invalid_no_digits);
    RUN_TEST(test_invalid_empty);
    RUN_TEST(test_invalid_null);
    RUN_TEST(test_invalid_sign_only);
    RUN_TEST(test_stops_at_non_digit);
    RUN_TEST(test_int_max);
    RUN_TEST(test_int_min);
    RUN_TEST(test_overflow_positive);
    RUN_TEST(test_overflow_negative);
    RUN_TEST(test_overflow_large);
    TEST_REPORT();
}
#endif
