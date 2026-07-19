// function_pointers2.c - Comparator and sorting
//
// A generic sort function uses void* pointers and a comparator callback
// to sort elements of ANY size. The comparator returns <0, 0, or >0.
//
// Three things to fix in bubble_sort:
// 1. Element addresses are computed with (int*)base — offsets are in
//    BYTES, so the base must be cast to char* / unsigned char*.
// 2. The swap buffer is a fixed unsigned char tmp[64]. A sort that
//    claims to handle any element size must size the buffer from
//    `size` (allocate it with malloc) — for bigger elements a fixed
//    buffer silently overflows. big_record_t below is bigger than 64
//    bytes for exactly this reason.
// 3. The contract: allocation can fail, so bubble_sort must return
//    int — 0 on success, -1 if the buffer allocation fails (leaving
//    the array untouched). The demo and the tests below are already
//    written against the new contract: a changed signature ripples to
//    every call site, so this file won't compile until the function
//    catches up with its callers.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>

int cmp_int_asc(const void *a, const void *b) {
    int ia = *(const int *)a;
    int ib = *(const int *)b;
    return (ia > ib) - (ia < ib);
}

int cmp_int_desc(const void *a, const void *b) {
    int ia = *(const int *)a;
    int ib = *(const int *)b;
    return (ib > ia) - (ib < ia);
}

// An element deliberately bigger than any "small types" buffer.
typedef struct {
    char label[80];
    int key;
} big_record_t;

int cmp_big_record_key(const void *a, const void *b) {
    int ka = ((const big_record_t *)a)->key;
    int kb = ((const big_record_t *)b)->key;
    return (ka > kb) - (ka < kb);
}

// TODO: make this return int (0 = sorted, -1 = allocation failure).
void bubble_sort(void *base, size_t count, size_t size,
                 int (*cmp)(const void *, const void *)) {
    unsigned char tmp[64];  // BUG: overflows for elements larger than 64 bytes
    for (size_t i = 0; i < count; i++) {
        for (size_t j = 0; j + 1 < count - i; j++) {
            // BUG: Using (int*)base instead of (char*)base for byte offsets.
            // void* arithmetic is not standard C; we must cast to char*.
            void *elem_j   = (int *)base + j * size;
            void *elem_j1  = (int *)base + (j + 1) * size;
            if (cmp(elem_j, elem_j1) > 0) {
                memcpy(tmp, elem_j, size);
                memcpy(elem_j, elem_j1, size);
                memcpy(elem_j1, tmp, size);
            }
        }
    }
}

#ifndef TEST
int main(void) {
    int nums[] = {5, 3, 8, 1, 9, 2};
    int len = sizeof(nums) / sizeof(nums[0]);

    if (bubble_sort(nums, (size_t)len, sizeof(int), cmp_int_asc) != 0) {
        printf("Sort failed!\n");
        return 1;
    }

    printf("Sorted ascending: ");
    for (int i = 0; i < len; i++) {
        printf("%d ", nums[i]);
    }
    printf("\n");

    big_record_t recs[3] = {
        {"gamma", 3},
        {"alpha", 1},
        {"beta", 2},
    };
    if (bubble_sort(recs, 3, sizeof(big_record_t), cmp_big_record_key) != 0) {
        printf("Sort failed!\n");
        return 1;
    }

    printf("Sorted records:   ");
    for (int i = 0; i < 3; i++) {
        printf("%s ", recs[i].label);
    }
    printf("\n");

    return 0;
}
#else
#include "clings_test.h"

TEST(test_sort_ascending) {
    int a[] = {5, 3, 8, 1, 9, 2};
    ASSERT_EQ(bubble_sort(a, 6, sizeof(int), cmp_int_asc), 0);
    ASSERT_EQ(a[0], 1); ASSERT_EQ(a[1], 2); ASSERT_EQ(a[2], 3);
    ASSERT_EQ(a[3], 5); ASSERT_EQ(a[4], 8); ASSERT_EQ(a[5], 9);
}

TEST(test_sort_descending) {
    int a[] = {5, 3, 8, 1, 9, 2};
    ASSERT_EQ(bubble_sort(a, 6, sizeof(int), cmp_int_desc), 0);
    ASSERT_EQ(a[0], 9); ASSERT_EQ(a[1], 8); ASSERT_EQ(a[2], 5);
    ASSERT_EQ(a[3], 3); ASSERT_EQ(a[4], 2); ASSERT_EQ(a[5], 1);
}

TEST(test_already_sorted) {
    int a[] = {1, 2, 3, 4, 5};
    ASSERT_EQ(bubble_sort(a, 5, sizeof(int), cmp_int_asc), 0);
    ASSERT_EQ(a[0], 1); ASSERT_EQ(a[1], 2); ASSERT_EQ(a[2], 3);
    ASSERT_EQ(a[3], 4); ASSERT_EQ(a[4], 5);
}

TEST(test_single_element) {
    int a[] = {42};
    ASSERT_EQ(bubble_sort(a, 1, sizeof(int), cmp_int_asc), 0);
    ASSERT_EQ(a[0], 42);
}

TEST(test_sort_large_elements) {
    /* Elements bigger than 64 bytes: a fixed swap buffer overflows,
     * only a buffer sized from `size` sorts these correctly. */
    big_record_t recs[4] = {
        {"delta", 4},
        {"beta", 2},
        {"alpha", 1},
        {"gamma", 3},
    };
    ASSERT_EQ(bubble_sort(recs, 4, sizeof(big_record_t), cmp_big_record_key), 0);
    ASSERT_EQ(recs[0].key, 1);
    ASSERT_STR_EQ(recs[0].label, "alpha");
    ASSERT_EQ(recs[1].key, 2);
    ASSERT_STR_EQ(recs[1].label, "beta");
    ASSERT_EQ(recs[3].key, 4);
    ASSERT_STR_EQ(recs[3].label, "delta");
}

int main(void) {
    RUN_TEST(test_sort_ascending);
    RUN_TEST(test_sort_descending);
    RUN_TEST(test_already_sorted);
    RUN_TEST(test_single_element);
    RUN_TEST(test_sort_large_elements);
    TEST_REPORT();
}
#endif
