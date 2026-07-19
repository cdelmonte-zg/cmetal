// ub4.c - Escaping the stack: pointers to dead locals
//
// A local variable lives only until its function returns. Returning a
// pointer to one hands the caller the address of storage that no longer
// exists -- using it is undefined behavior. It often "works" anyway:
// the bytes linger on the stack until the next call overwrites them,
// which makes this one of the most treacherous bugs in C.
//
// The blatant version is caught at compile time (gcc's
// -Wreturn-local-addr / clang's -Wreturn-stack-address, promoted to an
// error by -Werror), so this file does not build as shipped.
//
// TODO: make make_greeting return storage that survives the return:
// allocate the buffer on the heap, sized to fit the whole greeting.
// The caller becomes the owner and must free() it (the demo and the
// tests already do). Note that a `static` buffer is NOT enough here --
// see test_two_calls_are_independent.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Returns a newly built greeting string ("Hello, <name>!").
// The caller owns the returned buffer and must free() it.
char *make_greeting(const char *name) {
    char buf[64];  // BUG: buf dies when the function returns
    snprintf(buf, sizeof buf, "Hello, %s!", name);
    return buf;
}

#ifndef TEST
int main(void) {
    char *greeting = make_greeting("world");
    if (greeting == NULL) {
        return 1;
    }
    printf("%s\n", greeting);
    free(greeting);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_basic_greeting) {
    char *g = make_greeting("Ada");
    ASSERT(g != NULL);
    ASSERT_STR_EQ(g, "Hello, Ada!");
    free(g);
}

TEST(test_two_calls_are_independent) {
    /* A static buffer would survive the return, but the second call
     * would overwrite the first greeting. Each call must own its bytes. */
    char *a = make_greeting("Ada");
    char *b = make_greeting("Grace");
    ASSERT(a != NULL);
    ASSERT(b != NULL);
    ASSERT_STR_EQ(a, "Hello, Ada!");
    ASSERT_STR_EQ(b, "Hello, Grace!");
    ASSERT(a != b);
    free(a);
    free(b);
}

TEST(test_empty_name) {
    char *g = make_greeting("");
    ASSERT(g != NULL);
    ASSERT_STR_EQ(g, "Hello, !");
    free(g);
}

TEST(test_long_name) {
    /* 32 KiB of name: far beyond any plausible fixed-size buffer, so
     * the allocation must be sized to the actual greeting. */
    size_t name_len = 32u * 1024u;
    char *name = malloc(name_len + 1);
    ASSERT(name != NULL);
    memset(name, 'x', name_len);
    name[name_len] = '\0';

    char *g = make_greeting(name);
    ASSERT(g != NULL);
    ASSERT_EQ(strlen(g), strlen("Hello, !") + name_len);
    free(g);
    free(name);
}

int main(void) {
    RUN_TEST(test_basic_greeting);
    RUN_TEST(test_two_calls_are_independent);
    RUN_TEST(test_empty_name);
    RUN_TEST(test_long_name);
    TEST_REPORT();
}
#endif
