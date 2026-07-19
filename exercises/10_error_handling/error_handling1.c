// error_handling1.c - Return codes pattern
//
// Implement safe math functions that return error codes instead of
// crashing or producing undefined behavior.
//
// Error codes:
//   MATH_OK       (0)  — success
//   MATH_DIV_ZERO (-1) — division by zero attempted
//   MATH_OVERFLOW (-2) — result would overflow (e.g., INT_MIN / -1)
//
// BUG 1: safe_divide doesn't check for division by zero.
// BUG 2: safe_divide doesn't handle INT_MIN / -1 (signed overflow in C).
// BUG 3: safe_modulo has the same missing checks.
// Fix all three!

#include <stdio.h>
#include <limits.h>

enum math_error {
    MATH_OK       =  0,
    MATH_DIV_ZERO = -1,
    MATH_OVERFLOW = -2
};

// BUG: No check for b == 0 or overflow (INT_MIN / -1)
int safe_divide(int a, int b, int *result) {
    *result = a / b;
    return MATH_OK;
}

// BUG: No check for b == 0 or overflow (INT_MIN % -1)
int safe_modulo(int a, int b, int *result) {
    *result = a % b;
    return MATH_OK;
}

#ifndef TEST
int main(void) {
    int result;
    int err;

    err = safe_divide(10, 3, &result);
    printf("10 / 3 = %d (err=%d)\n", result, err);

    // NOTE: This would crash before fixing the bugs (division by zero).
    // Uncomment after you add the error checks:
    // err = safe_divide(10, 0, &result);
    // printf("10 / 0: err=%d\n", err);

    err = safe_modulo(10, 3, &result);
    printf("10 %% 3 = %d (err=%d)\n", result, err);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_divide_basic) {
    int result;
    int err = safe_divide(10, 3, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, 3);
}

TEST(test_divide_exact) {
    int result;
    int err = safe_divide(21, 7, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, 3);
}

TEST(test_divide_negative) {
    int result;
    int err = safe_divide(-15, 5, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, -3);
}

TEST(test_divide_by_zero) {
    int result = 999;
    int err = safe_divide(10, 0, &result);
    ASSERT_EQ(err, MATH_DIV_ZERO);
    // result should be untouched
    ASSERT_EQ(result, 999);
}

TEST(test_divide_int_min_by_neg1) {
    int result = 999;
    int err = safe_divide(INT_MIN, -1, &result);
    ASSERT_EQ(err, MATH_OVERFLOW);
    // result should be untouched
    ASSERT_EQ(result, 999);
}

TEST(test_divide_zero_numerator) {
    int result;
    int err = safe_divide(0, 5, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, 0);
}

TEST(test_modulo_basic) {
    int result;
    int err = safe_modulo(10, 3, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, 1);
}

TEST(test_modulo_by_zero) {
    int result = 999;
    int err = safe_modulo(10, 0, &result);
    ASSERT_EQ(err, MATH_DIV_ZERO);
    ASSERT_EQ(result, 999);
}

TEST(test_modulo_int_min_by_neg1) {
    int result = 999;
    int err = safe_modulo(INT_MIN, -1, &result);
    ASSERT_EQ(err, MATH_OVERFLOW);
    ASSERT_EQ(result, 999);
}

TEST(test_modulo_no_remainder) {
    int result;
    int err = safe_modulo(12, 4, &result);
    ASSERT_EQ(err, MATH_OK);
    ASSERT_EQ(result, 0);
}

int main(void) {
    RUN_TEST(test_divide_basic);
    RUN_TEST(test_divide_exact);
    RUN_TEST(test_divide_negative);
    RUN_TEST(test_divide_by_zero);
    RUN_TEST(test_divide_int_min_by_neg1);
    RUN_TEST(test_divide_zero_numerator);
    RUN_TEST(test_modulo_basic);
    RUN_TEST(test_modulo_by_zero);
    RUN_TEST(test_modulo_int_min_by_neg1);
    RUN_TEST(test_modulo_no_remainder);
    TEST_REPORT();
}
#endif
