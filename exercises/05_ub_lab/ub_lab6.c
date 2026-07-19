// ub_lab6.c - Out-of-bounds access by one (off-by-one)
//
// Accessing memory one byte past the end of a buffer is UNDEFINED
// BEHAVIOR. In a classic off-by-one error, the loop condition uses
// `<=` instead of `<`, reading one element past the end.
//
// This usually "works" because there's almost always *something* at
// that address, but AddressSanitizer will catch it.
//
// Fix: correct the loop bounds to stay within the allocated buffer.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Counts how many times character `c` appears in `str`.
// BUG: the loop goes one past the end of the string.
// TODO: Fix the loop bound so it doesn't read past the terminator.
int count_char(const char *str, char c) {
    int len = (int)strlen(str);
    int count = 0;
    // BUG: should be `i < len`, not `i <= len`.
    // When i == len, str[i] is the null terminator -- that's valid to read.
    // But wait -- the comparison also checks str[len], which is '\0'.
    // The REAL bug is below in find_last_index, see that function.
    for (int i = 0; i < len; i++) {
        if (str[i] == c) {
            count++;
        }
    }
    return count;
}

// Returns the index of the last occurrence of `c` in the buffer,
// or -1 if not found. `size` is the number of valid elements.
// BUG: the loop reads buf[size], which is one past the end!
// TODO: Fix the loop bound.
int find_last_index(const int *buf, int size, int c) {
    int last = -1;
    // BUG: `i <= size` reads one past the end when i == size
    for (int i = 0; i <= size; i++) {
        if (buf[i] == c) {
            last = i;
        }
    }
    return last;
}

// Copies `src` into a newly allocated buffer and returns it.
// BUG: allocates one byte too few (forgets the null terminator).
// TODO: Allocate strlen(src) + 1 bytes.
char *my_strdup(const char *src) {
    size_t len = strlen(src);
    // BUG: need len + 1 to include the null terminator
    char *copy = malloc(len);
    if (!copy) return NULL;
    // This memcpy writes `len` bytes, but the allocated buffer is only
    // `len` bytes, so the null terminator write is out-of-bounds.
    memcpy(copy, src, len + 1);
    return copy;
}

#ifndef TEST
int main(void) {
    const char *msg = "hello world";
    printf("'l' appears %d times in \"%s\"\n", count_char(msg, 'l'), msg);

    int data[] = {10, 20, 30, 20, 40};
    int idx = find_last_index(data, 5, 20);
    printf("Last index of 20: %d\n", idx);

    char *dup = my_strdup("test");
    if (dup) {
        printf("Duplicated: %s\n", dup);
        free(dup);
    }

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_count_char_basic) {
    ASSERT_EQ(count_char("hello", 'l'), 2);
    ASSERT_EQ(count_char("hello", 'h'), 1);
    ASSERT_EQ(count_char("hello", 'z'), 0);
}

TEST(test_count_char_empty) {
    ASSERT_EQ(count_char("", 'a'), 0);
}

TEST(test_find_last_basic) {
    int data[] = {10, 20, 30, 20, 40};
    ASSERT_EQ(find_last_index(data, 5, 20), 3);
    ASSERT_EQ(find_last_index(data, 5, 10), 0);
    ASSERT_EQ(find_last_index(data, 5, 40), 4);
}

TEST(test_find_last_not_found) {
    int data[] = {1, 2, 3};
    ASSERT_EQ(find_last_index(data, 3, 99), -1);
}

TEST(test_my_strdup_basic) {
    char *s = my_strdup("hello");
    ASSERT(s != NULL);
    ASSERT_STR_EQ(s, "hello");
    free(s);
}

TEST(test_my_strdup_empty) {
    char *s = my_strdup("");
    ASSERT(s != NULL);
    ASSERT_STR_EQ(s, "");
    free(s);
}

int main(void) {
    RUN_TEST(test_count_char_basic);
    RUN_TEST(test_count_char_empty);
    RUN_TEST(test_find_last_basic);
    RUN_TEST(test_find_last_not_found);
    RUN_TEST(test_my_strdup_basic);
    RUN_TEST(test_my_strdup_empty);
    TEST_REPORT();
}
#endif
