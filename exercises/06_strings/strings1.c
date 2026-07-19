// strings1.c - Safe string concatenation
//
// Implement safe_strcat() that concatenates src onto dst without overflowing
// the destination buffer. The dst_size parameter is the TOTAL size of the
// dst buffer (including the existing string and null terminator).
//
// Returns 0 on success, -1 if it would overflow (truncates to fit).
//
// BUG: The current implementation uses strcat() with no bounds checking,
// which causes a buffer overflow when src is too long. It also never
// returns -1. Replace the naive strcat() with proper bounded logic.

#include <stdio.h>
#include <string.h>

int safe_strcat(char *dst, size_t dst_size, const char *src) {
    // BUG: strcat() does not check bounds — it will happily write past
    // the end of dst if src is too long. This causes undefined behavior.
    // Also, this never returns -1 to signal overflow.
    // TODO: Replace with logic that:
    //   1. Computes remaining space (accounting for the null terminator)
    //   2. If src fits, copy it and return 0
    //   3. If src doesn't fit, truncate and return -1
    (void)dst_size; /* unused — but it shouldn't be! */
    strcat(dst, src);
    return 0;
}

#ifndef TEST
int main(void) {
    char buf[16] = "Hello";

    printf("Before: \"%s\"\n", buf);

    int rc = safe_strcat(buf, sizeof(buf), ", world!");
    printf("After:  \"%s\" (rc=%d)\n", buf, rc);

    rc = safe_strcat(buf, sizeof(buf), " This is way too long to fit.");
    printf("Trunc:  \"%s\" (rc=%d)\n", buf, rc);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_normal_concat) {
    char buf[32] = "Hello";
    int rc = safe_strcat(buf, sizeof(buf), ", world!");
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(buf, "Hello, world!");
}

TEST(test_empty_src) {
    char buf[16] = "Hello";
    int rc = safe_strcat(buf, sizeof(buf), "");
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(buf, "Hello");
}

TEST(test_empty_dst) {
    char buf[8] = "";
    int rc = safe_strcat(buf, sizeof(buf), "Hi");
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(buf, "Hi");
}

TEST(test_exact_fit) {
    // buf is 6 bytes: room for "Hello" (5 chars + null)
    char buf[6] = "He";
    // Appending "llo" should exactly fill: "Hello" + '\0' = 6 bytes
    int rc = safe_strcat(buf, sizeof(buf), "llo");
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(buf, "Hello");
}

TEST(test_overflow_truncates) {
    char buf[8] = "Hello";
    // buf has 8 bytes, "Hello" uses 5, so 2 chars + null can fit
    int rc = safe_strcat(buf, sizeof(buf), "!!!");
    ASSERT_EQ(rc, -1);
    // Should truncate: "Hello" + "!!" = "Hello!!" (7 chars + null = 8)
    ASSERT_STR_EQ(buf, "Hello!!");
}

TEST(test_no_room_at_all) {
    char buf[6] = "Hello";
    // Buffer is exactly full already (5 chars + null = 6 bytes)
    int rc = safe_strcat(buf, sizeof(buf), "X");
    ASSERT_EQ(rc, -1);
    ASSERT_STR_EQ(buf, "Hello");
}

int main(void) {
    RUN_TEST(test_normal_concat);
    RUN_TEST(test_empty_src);
    RUN_TEST(test_empty_dst);
    RUN_TEST(test_exact_fit);
    RUN_TEST(test_overflow_truncates);
    RUN_TEST(test_no_room_at_all);
    TEST_REPORT();
}
#endif
