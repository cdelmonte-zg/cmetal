// ub1.c - Signed integer overflow
//
// Signed integer overflow is UNDEFINED BEHAVIOR in C.
// The compiler assumes it never happens, and can optimize based on that.
//
// This program has a function that tries to detect overflow, but it does
// it WRONG because it relies on the overflow actually happening.
// Fix it to check BEFORE the overflow occurs.

#include <stdio.h>
#include <limits.h>

// TODO: Fix this function. It currently relies on signed overflow
// to detect the overflow, which is undefined behavior.
// Check BEFORE adding to see if overflow would occur.
int safe_add(int a, int b, int *result) {
    // BUG: signed overflow is UB! The compiler might optimize this check away.
    *result = a + b;
    if (a > 0 && b > 0 && *result < 0) {
        return -1;  // overflow
    }
    if (a < 0 && b < 0 && *result > 0) {
        return -1;  // underflow
    }
    return 0;  // ok
}

#ifndef TEST
int main(void) {
    int result;

    if (safe_add(INT_MAX, 1, &result) != 0) {
        printf("Overflow detected! INT_MAX + 1 would overflow.\n");
    } else {
        printf("Result: %d\n", result);
    }

    if (safe_add(100, 200, &result) == 0) {
        printf("100 + 200 = %d\n", result);
    }

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_normal_add) {
    int result;
    ASSERT_EQ(safe_add(1, 2, &result), 0);
    ASSERT_EQ(result, 3);
}

TEST(test_negative_add) {
    int result;
    ASSERT_EQ(safe_add(-5, -3, &result), 0);
    ASSERT_EQ(result, -8);
}

TEST(test_zero) {
    int result;
    ASSERT_EQ(safe_add(0, 0, &result), 0);
    ASSERT_EQ(result, 0);
}

TEST(test_overflow_detected) {
    int result;
    ASSERT_NE(safe_add(INT_MAX, 1, &result), 0);
    ASSERT_NE(safe_add(INT_MAX, INT_MAX, &result), 0);
}

TEST(test_underflow_detected) {
    int result;
    ASSERT_NE(safe_add(INT_MIN, -1, &result), 0);
    ASSERT_NE(safe_add(INT_MIN, INT_MIN, &result), 0);
}

TEST(test_edge_cases_no_overflow) {
    int result;
    ASSERT_EQ(safe_add(INT_MAX, 0, &result), 0);
    ASSERT_EQ(result, INT_MAX);
    ASSERT_EQ(safe_add(INT_MIN, 0, &result), 0);
    ASSERT_EQ(result, INT_MIN);
    ASSERT_EQ(safe_add(INT_MAX, -1, &result), 0);
    ASSERT_EQ(result, INT_MAX - 1);
}

int main(void) {
    RUN_TEST(test_normal_add);
    RUN_TEST(test_negative_add);
    RUN_TEST(test_zero);
    RUN_TEST(test_overflow_detected);
    RUN_TEST(test_underflow_detected);
    RUN_TEST(test_edge_cases_no_overflow);
    TEST_REPORT();
}
#endif
