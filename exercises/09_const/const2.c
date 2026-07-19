// const2.c - Pointer to const vs const pointer
//
// There are three ways to use const with pointers:
//   const int *p       — pointer to const int (can't modify *p, can reassign p)
//   int *const p       — const pointer to int (can modify *p, can't reassign p)
//   const int *const p — const pointer to const int (can't modify either)
//
// Implement two functions:
//   find_first()  — searches a const array for a target value, returns pointer
//   array_fill()  — fills every element of an array with a given value
//
// BUG 1: find_first only reads the array, but takes `int *arr` and
//         returns `int *`. The tests pass a `const int[]`: with -Werror
//         this doesn't even compile until parameter and return type
//         become `const int *`. This IS part of the function's
//         contract — callers holding const data rely on it. (Changing
//         a signature ripples to every call site; here the tests are
//         already written against the const-correct one.)
//
// BUG 2: array_fill walks the array by incrementing `arr` and, after
//         the loop, writes a sentinel through it — one element past the
//         end of the buffer. Fix the out-of-bounds write (the sanitizer
//         run catches it in the demo below).
//         Declaring the parameter `int *const arr` and indexing instead
//         of incrementing makes the compiler reject any future arr++ —
//         good self-defense. But know its limits: a top-level const on
//         a parameter is NOT part of the function's type (C11 6.7.6.3),
//         so no caller and no test can require it. It protects the
//         implementation; it doesn't change the contract.

#include <stdio.h>
#include <stddef.h>

// BUG: Should return `const int *` and take `const int *arr`
// since we only read the array, never write to it.
int *find_first(int *arr, int len, int target) {
    for (int i = 0; i < len; i++) {
        if (arr[i] == target) {
            return &arr[i];
        }
    }
    return NULL;
}

// BUG: The stray sentinel write below goes past the end of the buffer.
// Remove it, and switch to `int *const arr` with index-based access so
// the compiler itself forbids moving the pointer from now on.
void array_fill(int *arr, int len, int value) {
    for (int i = 0; i < len; i++) {
        *arr = value;
        arr++;
    }
    // BUG: arr now points one past the end — this writes out of bounds!
    *arr = 0;
}

#ifndef TEST
int main(void) {
    int data[] = {10, 20, 30, 40, 50};
    int *found = find_first(data, 5, 30);
    if (found) {
        printf("Found 30 at offset %td\n", found - data);
    }

    int buf[4] = {0};
    array_fill(buf, 4, 7);
    for (int i = 0; i < 4; i++) {
        printf("buf[%d] = %d\n", i, buf[i]);
    }

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_find_first_found) {
    const int arr[] = {10, 20, 30, 40, 50};
    const int *p = find_first(arr, 5, 30);
    ASSERT(p != NULL);
    ASSERT_EQ(*p, 30);
    ASSERT_EQ(p - arr, 2);
}

TEST(test_find_first_not_found) {
    const int arr[] = {1, 2, 3};
    const int *p = find_first(arr, 3, 99);
    ASSERT(p == NULL);
}

TEST(test_find_first_returns_first) {
    const int arr[] = {5, 5, 5};
    const int *p = find_first(arr, 3, 5);
    ASSERT(p != NULL);
    ASSERT_EQ(p - arr, 0);
}

TEST(test_array_fill_basic) {
    int arr[5] = {0};
    array_fill(arr, 5, 42);
    for (int i = 0; i < 5; i++) {
        ASSERT_EQ(arr[i], 42);
    }
}

TEST(test_array_fill_zero) {
    int arr[3] = {1, 2, 3};
    array_fill(arr, 3, 0);
    for (int i = 0; i < 3; i++) {
        ASSERT_EQ(arr[i], 0);
    }
}

TEST(test_array_fill_single) {
    int arr[1] = {99};
    array_fill(arr, 1, -1);
    ASSERT_EQ(arr[0], -1);
}

int main(void) {
    RUN_TEST(test_find_first_found);
    RUN_TEST(test_find_first_not_found);
    RUN_TEST(test_find_first_returns_first);
    RUN_TEST(test_array_fill_basic);
    RUN_TEST(test_array_fill_zero);
    RUN_TEST(test_array_fill_single);
    TEST_REPORT();
}
#endif
