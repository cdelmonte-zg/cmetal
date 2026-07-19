// pointers1.c - Array decay and sizeof
//
// When an array is passed to a function, it "decays" into a pointer.
// This means sizeof() no longer gives you the array size.
//
// Fix the array_sum() function so it correctly sums all elements.

#include <stdio.h>

// TODO: This function has a bug. It doesn't sum all elements because
// sizeof(arr) gives the pointer size, not the array size.
// Fix the function signature and the caller.
int array_sum(int arr[]) {
    int sum = 0;
    // BUG: sizeof(arr) here is sizeof(int*), not the array size!
    int len = sizeof(arr) / sizeof(arr[0]);
    for (int i = 0; i < len; i++) {
        sum += arr[i];
    }
    return sum;
}

#ifndef TEST
int main(void) {
    int numbers[] = {10, 20, 30, 40, 50};
    printf("Sum: %d\n", array_sum(numbers));
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_five_elements) {
    int a[] = {10, 20, 30, 40, 50};
    ASSERT_EQ(array_sum(a), 150);
}

TEST(test_single_element) {
    int b[] = {1};
    ASSERT_EQ(array_sum(b), 1);
}

TEST(test_negative_numbers) {
    int c[] = {-5, 5, -10, 10};
    ASSERT_EQ(array_sum(c), 0);
}

int main(void) {
    RUN_TEST(test_five_elements);
    RUN_TEST(test_single_element);
    RUN_TEST(test_negative_numbers);
    TEST_REPORT();
}
#endif
