// function_pointers3.c - Dispatch table (command pattern)
//
// A dispatch table maps string names to function pointers, enabling
// a command-pattern style lookup: given a name, find and execute
// the matching operation.
//
// Fix the bug in dispatch() so it correctly compares operation names.

#include <stdio.h>
#include <string.h>

struct operation {
    const char *name;
    int (*execute)(int, int);
};

static int op_add(int a, int b) {
    return a + b;
}

static int op_subtract(int a, int b) {
    return a - b;
}

static int op_multiply(int a, int b) {
    return a * b;
}

int dispatch(const struct operation *ops, int count, const char *name,
             int a, int b, int *result) {
    for (int i = 0; i < count; i++) {
        // BUG: Using == to compare strings compares pointers, not contents!
        // Two different string literals with the same text may have different addresses.
        if (ops[i].name == name) {
            *result = ops[i].execute(a, b);
            return 0;
        }
    }
    return -1;
}

#ifndef TEST
int main(void) {
    struct operation ops[] = {
        {"add",      op_add},
        {"subtract", op_subtract},
        {"multiply", op_multiply},
    };
    int num_ops = sizeof(ops) / sizeof(ops[0]);

    const char *commands[] = {"add", "multiply", "subtract", "modulo"};
    int num_cmds = sizeof(commands) / sizeof(commands[0]);

    for (int i = 0; i < num_cmds; i++) {
        int result = 0;
        int rc = dispatch(ops, num_ops, commands[i], 10, 3, &result);
        if (rc == 0) {
            printf("%s(10, 3) = %d\n", commands[i], result);
        } else {
            printf("%s: unknown operation\n", commands[i]);
        }
    }

    return 0;
}
#else
#include "clings_test.h"

TEST(test_dispatch_add) {
    struct operation ops[] = {
        {"add",      op_add},
        {"subtract", op_subtract},
        {"multiply", op_multiply},
    };
    /* A runtime copy of the name: same contents as the literal in ops,
       but a DIFFERENT address -- only strcmp() can match it. */
    char name[] = "add";
    int result = 0;
    int rc = dispatch(ops, 3, name, 7, 3, &result);
    ASSERT_EQ(rc, 0);
    ASSERT_EQ(result, 10);
}

TEST(test_dispatch_subtract) {
    struct operation ops[] = {
        {"add",      op_add},
        {"subtract", op_subtract},
        {"multiply", op_multiply},
    };
    char name[] = "subtract";
    int result = 0;
    int rc = dispatch(ops, 3, name, 20, 8, &result);
    ASSERT_EQ(rc, 0);
    ASSERT_EQ(result, 12);
}

TEST(test_dispatch_multiply) {
    struct operation ops[] = {
        {"add",      op_add},
        {"subtract", op_subtract},
        {"multiply", op_multiply},
    };
    char name[] = "multiply";
    int result = 0;
    int rc = dispatch(ops, 3, name, 6, 7, &result);
    ASSERT_EQ(rc, 0);
    ASSERT_EQ(result, 42);
}

TEST(test_dispatch_unknown) {
    struct operation ops[] = {
        {"add",      op_add},
        {"subtract", op_subtract},
        {"multiply", op_multiply},
    };
    char name[] = "modulo";
    int result = -999;
    int rc = dispatch(ops, 3, name, 10, 3, &result);
    ASSERT_EQ(rc, -1);
    ASSERT_EQ(result, -999);  /* result should be unchanged */
}

int main(void) {
    RUN_TEST(test_dispatch_add);
    RUN_TEST(test_dispatch_subtract);
    RUN_TEST(test_dispatch_multiply);
    RUN_TEST(test_dispatch_unknown);
    TEST_REPORT();
}
#endif
