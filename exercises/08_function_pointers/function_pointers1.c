// function_pointers1.c - Callback pattern (map/apply)
//
// Function pointers let you pass behavior as an argument.
// array_map() applies a transformation to each element in-place.
// array_apply() calls a function on each element without modifying it.
//
// Fix the bugs in array_map() so it correctly transforms the array.

#include <stdio.h>

int square(int x) {
    return x * x;
}

int negate(int x) {
    return -x;
}

int double_val(int x) {
    return x * 2;
}

void array_map(int *arr, int len, int (*fn)(int)) {
    // BUG: The result of fn() is computed but never stored back!
    for (int i = 0; i < len; i++) {
        fn(arr[i]);
    }
}

void array_apply(const int *arr, int len, void (*fn)(int)) {
    for (int i = 0; i < len; i++) {
        fn(arr[i]);
    }
}

#ifndef TEST
static void print_element(int x) {
    printf("%d ", x);
}
int main(void) {
    int numbers[] = {1, 2, 3, 4, 5};
    int len = sizeof(numbers) / sizeof(numbers[0]);

    printf("Original: ");
    array_apply(numbers, len, print_element);
    printf("\n");

    array_map(numbers, len, square);
    printf("Squared:  ");
    array_apply(numbers, len, print_element);
    printf("\n");

    array_map(numbers, len, negate);
    printf("Negated:  ");
    array_apply(numbers, len, print_element);
    printf("\n");

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_map_square) {
    int a[] = {1, 2, 3, 4, 5};
    array_map(a, 5, square);
    ASSERT_EQ(a[0], 1);
    ASSERT_EQ(a[1], 4);
    ASSERT_EQ(a[2], 9);
    ASSERT_EQ(a[3], 16);
    ASSERT_EQ(a[4], 25);
}

TEST(test_map_negate) {
    int a[] = {10, -20, 30};
    array_map(a, 3, negate);
    ASSERT_EQ(a[0], -10);
    ASSERT_EQ(a[1], 20);
    ASSERT_EQ(a[2], -30);
}

TEST(test_map_double) {
    int a[] = {0, 3, -7};
    array_map(a, 3, double_val);
    ASSERT_EQ(a[0], 0);
    ASSERT_EQ(a[1], 6);
    ASSERT_EQ(a[2], -14);
}

TEST(test_map_chain) {
    int a[] = {2, 3};
    array_map(a, 2, square);
    array_map(a, 2, negate);
    ASSERT_EQ(a[0], -4);
    ASSERT_EQ(a[1], -9);
}

int main(void) {
    RUN_TEST(test_map_square);
    RUN_TEST(test_map_negate);
    RUN_TEST(test_map_double);
    RUN_TEST(test_map_chain);
    TEST_REPORT();
}
#endif
