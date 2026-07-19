// ub_lab4.c - Signed integer overflow in multiplication
//
// Signed integer overflow is UNDEFINED BEHAVIOR in C. A factorial function
// using `int` will overflow silently for moderately large inputs (13! on
// 32-bit int). The result is UB, not just a wrong answer -- the compiler
// is free to assume it never happens.
//
// Fix: check for overflow before each multiplication, and return an error
// code when the result would exceed INT_MAX.

#include <stdio.h>
#include <limits.h>

// Computes n! and stores the result in *result.
// Returns 0 on success, -1 on overflow.
//
// BUG: For n >= 13 (on 32-bit int), the multiplication overflows,
// which is undefined behavior. The function never returns -1 because
// it doesn't actually check for overflow before multiplying.
// TODO: Before each multiplication, check whether multiplying would
// exceed INT_MAX. If so, return -1 without performing the multiply.
int factorial(int n, int *result) {
    if (n < 0) {
        return -1;
    }

    int fact = 1;
    for (int i = 2; i <= n; i++) {
        /* BUG: this can overflow for large i, which is UB */
        fact *= i;
    }

    *result = fact;
    return 0;
}

#ifndef TEST
int main(void) {
    for (int i = 0; i <= 15; i++) {
        int result;
        if (factorial(i, &result) == 0) {
            printf("%2d! = %d\n", i, result);
        } else {
            printf("%2d! = OVERFLOW\n", i);
        }
    }
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_zero) {
    int result;
    ASSERT_EQ(factorial(0, &result), 0);
    ASSERT_EQ(result, 1);
}

TEST(test_one) {
    int result;
    ASSERT_EQ(factorial(1, &result), 0);
    ASSERT_EQ(result, 1);
}

TEST(test_small_values) {
    int result;
    ASSERT_EQ(factorial(5, &result), 0);
    ASSERT_EQ(result, 120);

    ASSERT_EQ(factorial(6, &result), 0);
    ASSERT_EQ(result, 720);
}

TEST(test_twelve) {
    /* 12! = 479001600, fits in 32-bit int */
    int result;
    ASSERT_EQ(factorial(12, &result), 0);
    ASSERT_EQ(result, 479001600);
}

TEST(test_overflow_detected) {
    /* 13! = 6227020800, does NOT fit in 32-bit int */
    int result;
    ASSERT_EQ(factorial(13, &result), -1);
    ASSERT_EQ(factorial(20, &result), -1);
}

TEST(test_negative_input) {
    int result;
    ASSERT_EQ(factorial(-1, &result), -1);
}

int main(void) {
    RUN_TEST(test_zero);
    RUN_TEST(test_one);
    RUN_TEST(test_small_values);
    RUN_TEST(test_twelve);
    RUN_TEST(test_overflow_detected);
    RUN_TEST(test_negative_input);
    TEST_REPORT();
}
#endif
