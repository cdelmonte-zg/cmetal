// pointers2.c - Pointer arithmetic
//
// Pointer arithmetic in C advances by the size of the pointed-to type.
// If int *p points to an array of ints, p+1 advances by sizeof(int) bytes.
//
// Fix the swap and reverse functions using pointer arithmetic.

#include <stdio.h>

void swap(int *a, int *b) {
    // TODO: Swap the values pointed to by a and b.
    // Hint: you need a temporary variable.
    *a = *b;
    *b = *a;
}

void reverse_array(int *arr, int len) {
    // TODO: Reverse the array in-place using pointer arithmetic.
    // Use the swap() function above.
    // Hint: use two pointers, one from the start and one from the end.
    int *left = arr;
    int *right = arr + len;  // BUG: off by one
    while (left < right) {
        swap(left, right);
        left++;
        right--;
    }
}

#ifndef TEST
int main(void) {
    int arr[] = {1, 2, 3, 4, 5};
    int len = sizeof(arr) / sizeof(arr[0]);

    reverse_array(arr, len);

    printf("Reversed: ");
    for (int i = 0; i < len; i++) {
        printf("%d ", arr[i]);
    }
    printf("\n");

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_swap_direct) {
    /* swap() is part of the exercise: it must work on its own,
     * not just happen to be unused by a rewritten reverse_array. */
    int a = 1, b = 2;
    swap(&a, &b);
    ASSERT_EQ(a, 2);
    ASSERT_EQ(b, 1);
}

TEST(test_reverse_five) {
    int a[] = {1, 2, 3, 4, 5};
    reverse_array(a, 5);
    ASSERT_EQ(a[0], 5); ASSERT_EQ(a[1], 4); ASSERT_EQ(a[2], 3);
    ASSERT_EQ(a[3], 2); ASSERT_EQ(a[4], 1);
}

TEST(test_reverse_two) {
    int b[] = {10, 20};
    reverse_array(b, 2);
    ASSERT_EQ(b[0], 20); ASSERT_EQ(b[1], 10);
}

TEST(test_reverse_one) {
    int c[] = {42};
    reverse_array(c, 1);
    ASSERT_EQ(c[0], 42);
}

int main(void) {
    RUN_TEST(test_swap_direct);
    RUN_TEST(test_reverse_five);
    RUN_TEST(test_reverse_two);
    RUN_TEST(test_reverse_one);
    TEST_REPORT();
}
#endif
