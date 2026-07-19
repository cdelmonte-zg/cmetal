// error_handling3.c - Error context (building error messages)
//
// Implement a simple error reporting system that captures both an error
// code and a human-readable message. Then use it in a config-line parser.
//
// The parser handles lines of the form "key=value":
//   - Returns 0 on success, -1 on error (with details in the error struct)
//   - Must validate: non-NULL input, presence of '=', key/value fit in buffers
//
// BUG 1: error_set uses sprintf instead of vsnprintf — unsafe buffer overflow.
// BUG 2: parse_config_line doesn't check if key/value fit in their buffers.
// BUG 3: parse_config_line doesn't handle the case where '=' is missing.
// Fix all three!

#include <stdio.h>
#include <string.h>
#include <stdarg.h>

struct error {
    int code;
    char message[256];
};

// BUG: Uses sprintf which can overflow the message buffer.
// Should use vsnprintf with sizeof(err->message).
void error_set(struct error *err, int code, const char *fmt, ...) {
    err->code = code;
    va_list args;
    va_start(args, fmt);
    // BUG: sprintf does not limit output length — buffer overflow risk
    vsprintf(err->message, fmt, args);
    va_end(args);
}

void error_clear(struct error *err) {
    err->code = 0;
    err->message[0] = '\0';
}

// BUG: Doesn't check for missing '=' or buffer overflow on key/value.
int parse_config_line(const char *line, char *key, size_t key_size,
                      char *value, size_t value_size, struct error *err) {
    error_clear(err);

    // BUG: Missing NULL check
    // if (line == NULL) { ... }

    // Find the '=' delimiter
    const char *eq = NULL;
    for (int i = 0; line[i] != '\0'; i++) {
        if (line[i] == '=') {
            eq = &line[i];
            break;
        }
    }

    // BUG: Doesn't check if '=' was found
    // Should set error and return -1 if missing

    size_t key_len = (size_t)(eq - line);
    size_t val_len = strlen(eq + 1);

    // BUG: Doesn't check if key fits in buffer
    (void)key_size;  // unused — but it shouldn't be!
    memcpy(key, line, key_len);
    key[key_len] = '\0';

    // BUG: Doesn't check if value fits in buffer
    (void)value_size;  // unused — but it shouldn't be!
    memcpy(value, eq + 1, val_len);
    value[val_len] = '\0';

    return 0;
}

#ifndef TEST
int main(void) {
    struct error err;
    char key[32], value[32];

    // This valid call works even before the bugs are fixed:
    int rc = parse_config_line("name=alice", key, sizeof(key),
                               value, sizeof(value), &err);
    if (rc == 0) {
        printf("key=\"%s\" value=\"%s\"\n", key, value);
    } else {
        printf("Error %d: %s\n", err.code, err.message);
    }

    // NOTE: After fixing the bugs, uncomment these to see error handling:
    // rc = parse_config_line("bad line", key, sizeof(key),
    //                        value, sizeof(value), &err);
    // if (rc != 0) {
    //     printf("Error %d: %s\n", err.code, err.message);
    // }


    /* Long formatted messages must be truncated, never overflowed —
     * the sanitizer run exercises this path. */
    char big[600];
    memset(big, 'x', sizeof(big) - 1);
    big[sizeof(big) - 1] = '\0';
    error_set(&err, 9, "%s", big);
    printf("long message length: %zu\n", strlen(err.message));

    return 0;
}

#else
#include "clings_test.h"

TEST(test_parse_valid) {
    struct error err;
    char key[32], value[32];
    int rc = parse_config_line("name=alice", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(key, "name");
    ASSERT_STR_EQ(value, "alice");
}

TEST(test_parse_numeric_value) {
    struct error err;
    char key[32], value[32];
    int rc = parse_config_line("port=8080", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(key, "port");
    ASSERT_STR_EQ(value, "8080");
}

TEST(test_parse_empty_value) {
    struct error err;
    char key[32], value[32];
    int rc = parse_config_line("empty=", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, 0);
    ASSERT_STR_EQ(key, "empty");
    ASSERT_STR_EQ(value, "");
}

TEST(test_parse_null_input) {
    struct error err;
    char key[32], value[32];
    int rc = parse_config_line(NULL, key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, -1);
    ASSERT_NE(err.code, 0);
}

TEST(test_parse_missing_equals) {
    struct error err;
    char key[32], value[32];
    int rc = parse_config_line("no-equals-here", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, -1);
    ASSERT_NE(err.code, 0);
}

TEST(test_parse_key_too_long) {
    struct error err;
    char key[4], value[32];
    // Key is "longkey" (7 chars) but buffer is only 4
    int rc = parse_config_line("longkey=val", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, -1);
    ASSERT_NE(err.code, 0);
}

TEST(test_parse_value_too_long) {
    struct error err;
    char key[32], value[4];
    // Value is "longvalue" (9 chars) but buffer is only 4
    int rc = parse_config_line("k=longvalue", key, sizeof(key),
                               value, sizeof(value), &err);
    ASSERT_EQ(rc, -1);
    ASSERT_NE(err.code, 0);
}

TEST(test_error_set_message) {
    struct error err;
    error_set(&err, 42, "failed at step %d: %s", 3, "bad input");
    ASSERT_EQ(err.code, 42);
    ASSERT_STR_EQ(err.message, "failed at step 3: bad input");
}

TEST(test_error_set_long_message_truncated) {
    struct error err;
    char big[600];
    memset(big, 'x', sizeof(big) - 1);
    big[sizeof(big) - 1] = '\0';
    error_set(&err, 7, "%s", big);
    ASSERT_EQ(err.code, 7);
    /* message[256]: vsnprintf keeps it to 255 chars + NUL */
    ASSERT_EQ(strlen(err.message), sizeof(err.message) - 1);
}

TEST(test_error_clear) {
    struct error err;
    error_set(&err, 1, "some error");
    error_clear(&err);
    ASSERT_EQ(err.code, 0);
    ASSERT_STR_EQ(err.message, "");
}

int main(void) {
    RUN_TEST(test_parse_valid);
    RUN_TEST(test_parse_numeric_value);
    RUN_TEST(test_parse_empty_value);
    RUN_TEST(test_parse_null_input);
    RUN_TEST(test_parse_missing_equals);
    RUN_TEST(test_parse_key_too_long);
    RUN_TEST(test_parse_value_too_long);
    RUN_TEST(test_error_set_message);
    RUN_TEST(test_error_set_long_message_truncated);
    RUN_TEST(test_error_clear);
    TEST_REPORT();
}
#endif
