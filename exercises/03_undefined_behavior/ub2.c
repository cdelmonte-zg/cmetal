// ub2.c - Sequence points and unsequenced side effects
//
// Between two sequence points, C allows an object to be modified at most
// once -- and if it is modified, its old value may only be read to compute
// the new one (C11 6.5p2). Expressions like `x = x++` or `dst[i] = src[i++]`
// break that rule: the read and the increment are UNSEQUENCED, and the
// behavior is undefined. The same compiler can legally produce different
// results at different optimization levels.
//
// gcc and clang diagnose these patterns with -Wall, and clings compiles
// with -Werror, so this file does not even build until each expression
// is untangled into well-defined statements.

#include <stdio.h>

// Fills arr[0..n-1] with start, start+1, start+2, ...
// TODO: `arr[i++] = start + i` modifies i and reads it in the same
// expression with no sequence point in between. Split the store and
// the increment into separate statements.
void fill_sequence(int *arr, int n, int start) {
    int i = 0;
    while (i < n) {
        arr[i++] = start + i;  // BUG: unsequenced write and read of i
    }
}

// Copies src[0..n-1] into dst[0..n-1].
// TODO: same disease, different expression. Which i does dst[i] use,
// the old one or the incremented one? Neither: it's undefined.
void copy_ints(const int *src, int *dst, int n) {
    int i = 0;
    while (i < n) {
        dst[i] = src[i++];  // BUG: unsequenced write and read of i
    }
}

#ifndef TEST
int main(void) {
    int arr[5];
    fill_sequence(arr, 5, 10);
    printf("fill_sequence(5, start=10): ");
    for (int i = 0; i < 5; i++) {
        printf("%d ", arr[i]);
    }
    printf("\n");

    int src[4] = {1, 2, 3, 4};
    int dst[4] = {0, 0, 0, 0};
    copy_ints(src, dst, 4);
    printf("copy_ints: ");
    for (int i = 0; i < 4; i++) {
        printf("%d ", dst[i]);
    }
    printf("\n");

    return 0;
}
#else
#include "clings_test.h"

TEST(test_fill_from_zero) {
    int arr[4] = {-1, -1, -1, -1};
    fill_sequence(arr, 4, 0);
    ASSERT_EQ(arr[0], 0);
    ASSERT_EQ(arr[1], 1);
    ASSERT_EQ(arr[2], 2);
    ASSERT_EQ(arr[3], 3);
}

TEST(test_fill_with_offset) {
    int arr[3] = {-1, -1, -1};
    fill_sequence(arr, 3, 100);
    ASSERT_EQ(arr[0], 100);
    ASSERT_EQ(arr[1], 101);
    ASSERT_EQ(arr[2], 102);
}

TEST(test_fill_empty) {
    int arr[1] = {42};
    fill_sequence(arr, 0, 7);
    ASSERT_EQ(arr[0], 42);  /* untouched */
}

TEST(test_copy_basic) {
    int src[4] = {5, 6, 7, 8};
    int dst[4] = {0, 0, 0, 0};
    copy_ints(src, dst, 4);
    ASSERT_EQ(dst[0], 5);
    ASSERT_EQ(dst[1], 6);
    ASSERT_EQ(dst[2], 7);
    ASSERT_EQ(dst[3], 8);
}

TEST(test_copy_single) {
    int src[1] = {-9};
    int dst[1] = {0};
    copy_ints(src, dst, 1);
    ASSERT_EQ(dst[0], -9);
}

int main(void) {
    RUN_TEST(test_fill_from_zero);
    RUN_TEST(test_fill_with_offset);
    RUN_TEST(test_fill_empty);
    RUN_TEST(test_copy_basic);
    RUN_TEST(test_copy_single);
    TEST_REPORT();
}
#endif
