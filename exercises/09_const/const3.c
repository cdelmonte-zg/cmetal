// const3.c - Const with structs (immutable view)
//
// Implement a simple config module with proper const usage:
//   - config_create: allocates and initializes a config struct
//   - config_width: getter that takes a const pointer
//   - config_height: getter that takes a const pointer
//   - config_describe: formats a description into a caller-provided buffer
//   - config_destroy: frees the config
//
// BUG 1: config_create doesn't copy the whole title into the struct's
//         buffer — and the copy must be BOUNDED: titles longer than the
//         buffer are truncated, never overflowed (see the long-title
//         test and the sanitizer run of the demo).
// BUG 2: config_describe accidentally modifies the config struct and
//         takes a non-const pointer. The tests describe a config
//         through a `const struct config *`, so the signature must
//         become const — this IS part of the contract.
// Fix both bugs!

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct config {
    int width;
    int height;
    char title[64];
};

// BUG: Doesn't copy the title string into the struct's buffer.
// Instead it... well, there's a problem with how title is stored.
struct config *config_create(int w, int h, const char *title) {
    struct config *c = malloc(sizeof(struct config));
    if (!c) return NULL;
    c->width = w;
    c->height = h;
    // BUG: Only copies the first character of title
    c->title[0] = title[0];
    c->title[1] = '\0';
    return c;
}

int config_width(const struct config *c) {
    return c->width;
}

int config_height(const struct config *c) {
    return c->height;
}

// BUG: This function takes a non-const pointer and accidentally
// modifies the config. It should take a const pointer and not
// modify the struct.
void config_describe(struct config *c, char *buf, size_t buf_size) {
    // BUG: accidentally zeroes width and height
    c->width = 0;
    c->height = 0;
    snprintf(buf, buf_size, "%s: %dx%d", c->title, c->width, c->height);
}

void config_destroy(struct config *c) {
    free(c);
}

#ifndef TEST
int main(void) {
    struct config *cfg = config_create(1920, 1080, "Main Window");
    if (!cfg) return 1;

    char buf[128];
    config_describe(cfg, buf, sizeof(buf));
    printf("%s\n", buf);
    printf("Width: %d, Height: %d\n", config_width(cfg), config_height(cfg));

    config_destroy(cfg);

    /* A title longer than the struct's buffer must be truncated, never
     * overflowed — the sanitizer run exercises this path. */
    char long_title[200];
    memset(long_title, 'T', sizeof(long_title) - 1);
    long_title[sizeof(long_title) - 1] = '\0';
    struct config *big = config_create(1, 2, long_title);
    if (!big) return 1;
    printf("long title stored: %zu chars\n", strlen(big->title));
    config_destroy(big);
    return 0;
}
#else
#include "clings_test.h"

TEST(test_config_create) {
    struct config *c = config_create(800, 600, "Test");
    ASSERT(c != NULL);
    ASSERT_EQ(config_width(c), 800);
    ASSERT_EQ(config_height(c), 600);
    config_destroy(c);
}

TEST(test_config_title_copied) {
    char title[32];
    snprintf(title, sizeof(title), "Dynamic");
    struct config *c = config_create(100, 200, title);
    // Modify the original — config should have its own copy
    title[0] = 'X';
    char buf[128];
    config_describe(c, buf, sizeof(buf));
    ASSERT_STR_EQ(buf, "Dynamic: 100x200");
    config_destroy(c);
}

TEST(test_config_describe_format) {
    struct config *c = config_create(1920, 1080, "Main");
    char buf[128];
    config_describe(c, buf, sizeof(buf));
    ASSERT_STR_EQ(buf, "Main: 1920x1080");
    config_destroy(c);
}

TEST(test_config_describe_does_not_modify) {
    struct config *c = config_create(640, 480, "Window");
    char buf[128];
    config_describe(c, buf, sizeof(buf));
    // After describe, the config should be unchanged
    ASSERT_EQ(config_width(c), 640);
    ASSERT_EQ(config_height(c), 480);
    config_destroy(c);
}

TEST(test_describe_accepts_const_config) {
    /* A read-only view of the config must be describable: this forces
     * config_describe to take a const struct config *. */
    struct config *c = config_create(10, 20, "RO");
    const struct config *view = c;
    char buf[128];
    config_describe(view, buf, sizeof(buf));
    ASSERT_STR_EQ(buf, "RO: 10x20");
    config_destroy(c);
}

TEST(test_config_long_title_truncated) {
    /* The title buffer is 64 bytes: a 199-char title must be cut to 63
     * chars + NUL, not written past the end of the struct. */
    char long_title[200];
    memset(long_title, 'T', sizeof(long_title) - 1);
    long_title[sizeof(long_title) - 1] = '\0';
    struct config *c = config_create(1, 2, long_title);
    ASSERT(c != NULL);
    char buf[300];
    config_describe(c, buf, sizeof(buf));
    ASSERT_EQ(strlen(buf), 63u + strlen(": 1x2"));
    config_destroy(c);
}

TEST(test_config_small_buffer) {
    struct config *c = config_create(1920, 1080, "Main");
    char buf[8];
    config_describe(c, buf, sizeof(buf));
    // Should be truncated but not overflow
    ASSERT(strlen(buf) == 7);
    config_destroy(c);
}

int main(void) {
    RUN_TEST(test_config_create);
    RUN_TEST(test_config_title_copied);
    RUN_TEST(test_config_describe_format);
    RUN_TEST(test_config_describe_does_not_modify);
    RUN_TEST(test_describe_accepts_const_config);
    RUN_TEST(test_config_long_title_truncated);
    RUN_TEST(test_config_small_buffer);
    TEST_REPORT();
}
#endif
