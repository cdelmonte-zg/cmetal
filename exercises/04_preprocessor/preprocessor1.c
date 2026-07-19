// preprocessor1.c - Macro stringification and token pasting
//
// The C preprocessor has two powerful operators:
//   # (stringify) - turns a macro argument into a string literal
//   ## (token paste) - concatenates two tokens into one
//
// Fix the macros below to make the tests pass.

#include <stdio.h>
#include <string.h>

// TODO: Fix this macro. It should turn its argument into a string.
// Example: STRINGIFY(hello) should produce "hello"
#define STRINGIFY(x) x

// TODO: Fix this macro. It should concatenate two tokens.
// Example: CONCAT(foo, bar) should produce the identifier foobar
#define CONCAT(a, b) a b

// TODO: Fix this macro. It should format "name = <value>" into buf,
// where name is the VARIABLE'S NAME: with int x = 42,
// FORMAT_VAR(buf, size, x) must produce "x = 42".
// Hint: you need STRINGIFY and snprintf with %d format.
#define FORMAT_VAR(buf, size, var) snprintf(buf, size, "%s = %d", "var", var)

#ifndef TEST
int main(void) {
    int my_value = 42;
    char buf[64];
    FORMAT_VAR(buf, sizeof buf, my_value);
    printf("%s\n", buf);

    printf("STRINGIFY test: %s\n", STRINGIFY(hello_world));

    int foobar = 99;
    printf("CONCAT test: %d\n", CONCAT(foo, bar));

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_stringify) {
    ASSERT_STR_EQ(STRINGIFY(hello), "hello");
    ASSERT_STR_EQ(STRINGIFY(foo_bar), "foo_bar");
}

TEST(test_concat) {
    int foobar = 42;
    ASSERT_EQ(CONCAT(foo, bar), 42);

    int test123 = 99;
    ASSERT_EQ(CONCAT(test, 123), 99);
}

TEST(test_format_var) {
    int some_var = 7;
    char buf[64];
    FORMAT_VAR(buf, sizeof buf, some_var);
    ASSERT_STR_EQ(buf, "some_var = 7");
}

int main(void) {
    RUN_TEST(test_stringify);
    RUN_TEST(test_concat);
    RUN_TEST(test_format_var);
    TEST_REPORT();
}
#endif
