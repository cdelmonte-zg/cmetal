// memory2.c - Realloc patterns
//
// This program implements a dynamic integer array ("DynArray") that grows
// as elements are pushed. There are bugs in how realloc is used, in how
// the struct is initialized — and in the contract itself: a void push
// has no way to tell the caller its value was silently dropped.
//
// The target contract of dynarray_push: return 0 on success and -1 if
// growing the array fails — and on failure the array is left EXACTLY
// as it was (same items, same count, same capacity).

#include <stdio.h>
#include <stdlib.h>
#include "clings_alloc.h"

typedef struct {
    int *items;
    int count;
    int capacity;
} DynArray;

DynArray *dynarray_create(int initial_cap) {
    DynArray *da = malloc(sizeof(DynArray));
    if (!da) return NULL;
    da->items = malloc(sizeof(int) * (size_t)initial_cap);
    if (!da->items) {
        free(da);
        return NULL;
    }
    // BUG: capacity and count are not initialized!
    // What should they be set to?
    return da;
}

// TODO: growth can fail, so push must be able to say so. Change the
// contract to:
//     int dynarray_push(DynArray *da, int value);
// returning 0 on success and -1 when growing fails, leaving the array
// untouched. Note: a changed signature ripples to every call site —
// the demo and the tests below are already written against the new
// contract, so this file won't even compile until the function
// catches up with its callers.
void dynarray_push(DynArray *da, int value) {
    if (da->count >= da->capacity) {
        int new_cap = da->capacity * 2;
        // BUG: If realloc fails, it returns NULL but the original pointer
        // is overwritten — we lose our data and leak the old block. Worse:
        // we then write through the NULL pointer anyway.
        // Use a temporary pointer, and return -1 without touching the
        // array if the allocation failed.
        da->items = CLINGS_REALLOC(da->items, sizeof(int) * (size_t)new_cap);
        da->capacity = new_cap;
    }
    da->items[da->count] = value;
    da->count++;
}

int dynarray_get(const DynArray *da, int index) {
    if (index < 0 || index >= da->count) {
        return -1;
    }
    return da->items[index];
}

void dynarray_destroy(DynArray *da) {
    if (!da) return;
    free(da->items);
    free(da);
}

#ifndef TEST
int main(void) {
    DynArray *arr = dynarray_create(2);
    if (!arr) {
        printf("Allocation failed!\n");
        return 1;
    }

    int values[] = {10, 20, 30, 40, 50};
    for (int i = 0; i < 5; i++) {
        if (dynarray_push(arr, values[i]) != 0) {
            printf("Push failed!\n");
            dynarray_destroy(arr);
            return 1;
        }
    }

    printf("DynArray contents:\n");
    for (int i = 0; i < arr->count; i++) {
        printf("  [%d] = %d\n", i, dynarray_get(arr, i));
    }

    dynarray_destroy(arr);
    return 0;
}
#else
#include "clings_test.h"

TEST(test_create) {
    DynArray *da = dynarray_create(4);
    ASSERT(da != NULL);
    ASSERT(da->items != NULL);
    ASSERT_EQ(da->count, 0);
    ASSERT_EQ(da->capacity, 4);
    dynarray_destroy(da);
}

TEST(test_push_and_get) {
    DynArray *da = dynarray_create(4);
    ASSERT_EQ(dynarray_push(da, 10), 0);
    ASSERT_EQ(dynarray_push(da, 20), 0);
    ASSERT_EQ(dynarray_push(da, 30), 0);
    ASSERT_EQ(da->count, 3);
    ASSERT_EQ(dynarray_get(da, 0), 10);
    ASSERT_EQ(dynarray_get(da, 1), 20);
    ASSERT_EQ(dynarray_get(da, 2), 30);
    dynarray_destroy(da);
}

TEST(test_grow_beyond_capacity) {
    DynArray *da = dynarray_create(2);
    ASSERT_EQ(da->capacity, 2);
    ASSERT_EQ(dynarray_push(da, 100), 0);
    ASSERT_EQ(dynarray_push(da, 200), 0);
    // This push triggers a realloc; success must be reported here too.
    ASSERT_EQ(dynarray_push(da, 300), 0);
    ASSERT_EQ(da->count, 3);
    ASSERT(da->capacity >= 3);
    ASSERT_EQ(dynarray_get(da, 0), 100);
    ASSERT_EQ(dynarray_get(da, 1), 200);
    ASSERT_EQ(dynarray_get(da, 2), 300);
    dynarray_destroy(da);
}

TEST(test_get_out_of_bounds) {
    DynArray *da = dynarray_create(2);
    ASSERT_EQ(dynarray_push(da, 42), 0);
    ASSERT_EQ(dynarray_get(da, 0), 42);
    ASSERT_EQ(dynarray_get(da, 1), -1);
    ASSERT_EQ(dynarray_get(da, -1), -1);
    dynarray_destroy(da);
}

TEST(test_push_failure_leaves_array_untouched) {
    DynArray *da = dynarray_create(2);
    ASSERT_EQ(dynarray_push(da, 1), 0);
    ASSERT_EQ(dynarray_push(da, 2), 0);
    int *items_before = da->items;
    clings_fail_next_alloc();          /* the growth realloc will fail */
    ASSERT_EQ(dynarray_push(da, 3), -1);
    ASSERT_EQ(da->count, 2);
    ASSERT_EQ(da->capacity, 2);
    ASSERT(da->items == items_before);
    ASSERT_EQ(dynarray_get(da, 0), 1);
    ASSERT_EQ(dynarray_get(da, 1), 2);
    /* the array must still work after the failed push */
    ASSERT_EQ(dynarray_push(da, 3), 0);
    ASSERT_EQ(dynarray_get(da, 2), 3);
    dynarray_destroy(da);
}

int main(void) {
    RUN_TEST(test_create);
    RUN_TEST(test_push_and_get);
    RUN_TEST(test_grow_beyond_capacity);
    RUN_TEST(test_get_out_of_bounds);
    RUN_TEST(test_push_failure_leaves_array_untouched);
    TEST_REPORT();
}
#endif
