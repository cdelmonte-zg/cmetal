// error_handling2.c - Error propagation (cleanup on error)
//
// Implement read_pair() which parses a string of the form "num1,num2"
// and returns the two integers through output parameters.
//
// Return codes:
//   0  — success
//  -1  — NULL or empty input
//  -2  — missing comma delimiter
//  -3  — invalid first number
//  -4  — invalid second number
//
// The function must validate each step and return the correct error code.
//
// BUG 1: Missing NULL/empty check on input.
// BUG 2: Doesn't properly detect missing comma.
// BUG 3: Doesn't validate that the parsed numbers are actual numbers
//         (the manual parser accepts anything).
// Fix all bugs so every error path returns the right code!

#include <stdio.h>
#include <string.h>
#include <limits.h>

/* parse_int's limit arithmetic and the INT_MIN boundary test assume a
 * 32-bit two's-complement int, per project convention (see ub3). */
_Static_assert(INT_MAX == 2147483647 && INT_MIN == (-2147483647 - 1),
               "error_handling2 requires a 32-bit two's-complement int");

// Helper: returns 1 if s contains only digits (with optional leading '-'), 0 otherwise
static int is_valid_int(const char *s, int len) {
    if (len <= 0) return 0;
    int start = 0;
    if (s[0] == '-') {
        if (len == 1) return 0;  // just "-" is not valid
        start = 1;
    }
    for (int i = start; i < len; i++) {
        if (s[i] < '0' || s[i] > '9') return 0;
    }
    return 1;
}

// Helper: string-to-int with RANGE checking (digit validation is
// is_valid_int's job). Returns 0 and stores the value through out, or
// -1 if the number does not fit in an int. Writes *out only on success.
//
// The overflow check happens BEFORE each multiply-add, in unsigned
// arithmetic: checking after the operation would itself be UB once the
// accumulator overflows — no matter how wide the accumulator, a long
// enough digit string overflows it.
static int parse_int(const char *s, int len, int *out) {
    unsigned int result = 0;
    int negative = s[0] == '-';
    int start = negative ? 1 : 0;
    /* |INT_MIN| is one more than INT_MAX on two's complement */
    unsigned int limit = negative ? (unsigned int)INT_MAX + 1u
                                  : (unsigned int)INT_MAX;
    for (int i = start; i < len; i++) {
        unsigned int digit = (unsigned int)(s[i] - '0');
        if (result > (limit - digit) / 10u) {
            return -1;  /* result * 10 + digit would exceed the limit */
        }
        result = result * 10u + digit;
    }
    if (negative) {
        *out = result == (unsigned int)INT_MAX + 1u ? INT_MIN
                                                    : -(int)result;
    } else {
        *out = (int)result;
    }
    return 0;
}

int read_pair(const char *input, int *a, int *b) {
    // BUG: Missing NULL/empty check — should return -1

    // Find the comma
    const char *comma = NULL;
    for (int i = 0; input[i] != '\0'; i++) {
        if (input[i] == ',') {
            comma = &input[i];
            break;
        }
    }

    // BUG: Doesn't check if comma was found — should return -2
    // if (comma == NULL) return -2;

    int first_len = (int)(comma - input);
    int second_len = (int)strlen(comma + 1);

    // BUG: Calls is_valid_int but ignores the result!
    // Should return -3 if the first number is invalid.
    is_valid_int(input, first_len);
    if (parse_int(input, first_len, a) != 0) return -3;  /* range check */

    // BUG: Same problem — ignores validation result.
    // Should return -4 if the second number is invalid.
    is_valid_int(comma + 1, second_len);
    if (parse_int(comma + 1, second_len, b) != 0) return -4;  /* range check */

    return 0;
}

#ifndef TEST
int main(void) {
    int a, b;
    int err;

    // This valid call works even before the bugs are fixed:
    err = read_pair("42,17", &a, &b);
    printf("\"42,17\" -> a=%d, b=%d (err=%d)\n", a, b, err);

    // NOTE: These calls crash before the bugs are fixed (null deref).
    // Uncomment after you add proper error checking:
    // err = read_pair("hello", &a, &b);
    // printf("\"hello\" -> err=%d\n", err);
    //
    // err = read_pair("", &a, &b);
    // printf("\"\" -> err=%d\n", err);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_valid_pair) {
    int a = 0, b = 0;
    int err = read_pair("42,17", &a, &b);
    ASSERT_EQ(err, 0);
    ASSERT_EQ(a, 42);
    ASSERT_EQ(b, 17);
}

TEST(test_negative_numbers) {
    int a = 0, b = 0;
    int err = read_pair("-5,10", &a, &b);
    ASSERT_EQ(err, 0);
    ASSERT_EQ(a, -5);
    ASSERT_EQ(b, 10);
}

TEST(test_both_negative) {
    int a = 0, b = 0;
    int err = read_pair("-3,-7", &a, &b);
    ASSERT_EQ(err, 0);
    ASSERT_EQ(a, -3);
    ASSERT_EQ(b, -7);
}

TEST(test_zeros) {
    int a = 99, b = 99;
    int err = read_pair("0,0", &a, &b);
    ASSERT_EQ(err, 0);
    ASSERT_EQ(a, 0);
    ASSERT_EQ(b, 0);
}

TEST(test_null_input) {
    int a = 99, b = 99;
    int err = read_pair(NULL, &a, &b);
    ASSERT_EQ(err, -1);
    // a and b should be untouched
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_empty_input) {
    int a = 99, b = 99;
    int err = read_pair("", &a, &b);
    ASSERT_EQ(err, -1);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_missing_comma) {
    int a = 99, b = 99;
    int err = read_pair("12345", &a, &b);
    ASSERT_EQ(err, -2);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_invalid_first_number) {
    int a = 99, b = 99;
    int err = read_pair("abc,5", &a, &b);
    ASSERT_EQ(err, -3);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_invalid_second_number) {
    int a = 99, b = 99;
    int err = read_pair("5,xyz", &a, &b);
    ASSERT_EQ(err, -4);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_number_too_big_for_int) {
    int a = 99, b = 99;
    /* 10 digits: all digits (passes is_valid_int), but does not fit in
     * an int — parsing must fail cleanly instead of overflowing. */
    int err = read_pair("9999999999,1", &a, &b);
    ASSERT_EQ(err, -3);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_extremely_large_first_number) {
    int a = 99, b = 99;
    /* Long enough to overflow ANY fixed-width accumulator: the range
     * check must reject it before the arithmetic, not after. */
    int err = read_pair(
        "999999999999999999999999999999999999999999999999,1", &a, &b);
    ASSERT_EQ(err, -3);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_int_boundaries) {
    int a = 0, b = 0;
    ASSERT_EQ(read_pair("2147483647,-2147483648", &a, &b), 0);
    ASSERT_EQ(a, INT_MAX);
    ASSERT_EQ(b, INT_MIN);
}

TEST(test_empty_before_comma) {
    int a = 99, b = 99;
    int err = read_pair(",5", &a, &b);
    ASSERT_EQ(err, -3);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

TEST(test_empty_after_comma) {
    int a = 99, b = 99;
    int err = read_pair("5,", &a, &b);
    ASSERT_EQ(err, -4);
    ASSERT_EQ(a, 99);
    ASSERT_EQ(b, 99);
}

int main(void) {
    RUN_TEST(test_valid_pair);
    RUN_TEST(test_negative_numbers);
    RUN_TEST(test_both_negative);
    RUN_TEST(test_zeros);
    RUN_TEST(test_null_input);
    RUN_TEST(test_empty_input);
    RUN_TEST(test_missing_comma);
    RUN_TEST(test_invalid_first_number);
    RUN_TEST(test_invalid_second_number);
    RUN_TEST(test_number_too_big_for_int);
    RUN_TEST(test_extremely_large_first_number);
    RUN_TEST(test_int_boundaries);
    RUN_TEST(test_empty_before_comma);
    RUN_TEST(test_empty_after_comma);
    TEST_REPORT();
}
#endif
