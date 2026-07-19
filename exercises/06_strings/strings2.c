// strings2.c - String tokenizer (manual strtok)
//
// Implement next_token() that extracts the next token from a string, splitting
// on a single delimiter character. The *cursor pointer is advanced past the
// delimiter after each call.
//
// Returns 1 if a token was found (even if empty), 0 if end of string.
//
// There is a BUG: empty tokens (two consecutive delimiters) are not handled
// correctly. Find and fix it!

#include <stdio.h>
#include <string.h>

int next_token(const char **cursor, char delim, char *buf, size_t buf_size) {
    if (*cursor == NULL) {
        buf[0] = '\0';
        return 0;
    }

    const char *start = *cursor;
    const char *end = start;

    // BUG: This skips over delimiters at the start, which means
    // empty tokens between consecutive delimiters are lost.
    while (*end == delim) {
        end++;
    }
    start = end;

    // Find the next delimiter or end of string
    while (*end != '\0' && *end != delim) {
        end++;
    }

    // Copy token into buf
    size_t len = (size_t)(end - start);
    if (len >= buf_size) {
        len = buf_size - 1;
    }
    memcpy(buf, start, len);
    buf[len] = '\0';

    // Advance cursor past the delimiter, or signal end
    if (*end == delim) {
        *cursor = end + 1;
    } else {
        *cursor = NULL;
    }

    return 1;
}

#ifndef TEST
int main(void) {
    const char *input = "hello,world,,foo";
    const char *cursor = input;
    char token[64];

    printf("Splitting \"%s\" by ',':\n", input);
    while (next_token(&cursor, ',', token, sizeof(token))) {
        printf("  token: \"%s\"\n", token);
    }

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_basic_split) {
    const char *cursor = "hello,world,foo";
    char tok[64];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "hello");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "world");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "foo");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

TEST(test_empty_tokens) {
    const char *cursor = "a,,b";
    char tok[64];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "a");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "b");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

TEST(test_no_delimiter) {
    const char *cursor = "single";
    char tok[64];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "single");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

TEST(test_trailing_delimiter) {
    const char *cursor = "a,b,";
    char tok[64];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "a");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "b");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

TEST(test_leading_delimiter) {
    const char *cursor = ",x";
    char tok[64];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "x");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

TEST(test_small_buffer) {
    const char *cursor = "longtoken,short";
    char tok[5];

    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    // "longtoken" truncated to 4 chars + null
    ASSERT_STR_EQ(tok, "long");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 1);
    ASSERT_STR_EQ(tok, "shor");
    ASSERT_EQ(next_token(&cursor, ',', tok, sizeof(tok)), 0);
}

int main(void) {
    RUN_TEST(test_basic_split);
    RUN_TEST(test_empty_tokens);
    RUN_TEST(test_no_delimiter);
    RUN_TEST(test_trailing_delimiter);
    RUN_TEST(test_leading_delimiter);
    RUN_TEST(test_small_buffer);
    TEST_REPORT();
}
#endif
