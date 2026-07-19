// structs2.c - Opaque pointer pattern (encapsulation)
//
// An "opaque pointer" hides the internal representation of a data structure
// behind a typedef. Users interact only through functions, never touching
// the struct fields directly. This is C's version of encapsulation.
//
// Implement a stack of ints using a dynamic array internally.
// Fix the bugs in stack_push() and stack_pop(). Part of the fix is the
// CONTRACT of stack_push: growth can fail, so push must return int —
// 0 on success, -1 when growing fails (leaving the stack untouched).

#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int *data;
    int top;       // index of next free slot (also the count)
    int capacity;
} Stack;

Stack *stack_create(int capacity) {
    Stack *s = malloc(sizeof(Stack));
    if (!s) return NULL;
    s->data = malloc(sizeof(int) * (size_t)capacity);
    if (!s->data) {
        free(s);
        return NULL;
    }
    s->top = 0;
    s->capacity = capacity;
    return s;
}

// TODO: change the contract to
//     int stack_push(Stack *s, int value);
// returning 0 on success and -1 when growing fails, leaving the stack
// untouched. Note: a changed signature ripples to every call site —
// the demo and the tests below are already written against the new
// contract, so this file won't compile until the function catches up.
void stack_push(Stack *s, int value) {
    if (s->top >= s->capacity) {
        s->capacity *= 2;
        // BUG: realloc result is not checked and is assigned directly.
        // If realloc fails, the original pointer is lost (memory leak)
        // and s->data becomes NULL, causing a crash on the next write.
        //
        // TODO: Use a temporary pointer, and return -1 without touching
        // the stack if the allocation failed.
        s->data = realloc(s->data, sizeof(int) * (size_t)s->capacity);
    }
    s->data[s->top] = value;
    s->top++;
}

// BUG: stack_pop does not check whether the stack is empty.
// Popping from an empty stack causes undefined behavior (reading
// uninitialized or out-of-bounds memory).
//
// TODO: Add a check for empty stack. If empty, return -1 as a sentinel.
int stack_pop(Stack *s) {
    s->top--;
    return s->data[s->top];
}

int stack_peek(const Stack *s) {
    if (s->top == 0) return -1;
    return s->data[s->top - 1];
}

int stack_size(const Stack *s) {
    return s->top;
}

void stack_destroy(Stack *s) {
    if (!s) return;
    free(s->data);
    free(s);
}

#ifndef TEST
int main(void) {
    Stack *s = stack_create(4);
    if (stack_push(s, 10) != 0 || stack_push(s, 20) != 0 ||
        stack_push(s, 30) != 0) {
        printf("Push failed!\n");
        stack_destroy(s);
        return 1;
    }

    printf("size: %d\n", stack_size(s));
    printf("peek: %d\n", stack_peek(s));
    printf("pop:  %d\n", stack_pop(s));
    printf("pop:  %d\n", stack_pop(s));
    printf("size: %d\n", stack_size(s));

    stack_destroy(s);
    return 0;
}
#else
#include "clings_test.h"

TEST(test_create_and_destroy) {
    Stack *s = stack_create(4);
    ASSERT(s != NULL);
    ASSERT_EQ(stack_size(s), 0);
    stack_destroy(s);
}

TEST(test_push_pop) {
    Stack *s = stack_create(4);
    ASSERT_EQ(stack_push(s, 10), 0);
    ASSERT_EQ(stack_push(s, 20), 0);
    ASSERT_EQ(stack_push(s, 30), 0);
    ASSERT_EQ(stack_size(s), 3);
    ASSERT_EQ(stack_pop(s), 30);
    ASSERT_EQ(stack_pop(s), 20);
    ASSERT_EQ(stack_pop(s), 10);
    ASSERT_EQ(stack_size(s), 0);
    stack_destroy(s);
}

TEST(test_peek) {
    Stack *s = stack_create(4);
    ASSERT_EQ(stack_peek(s), -1);
    ASSERT_EQ(stack_push(s, 42), 0);
    ASSERT_EQ(stack_peek(s), 42);
    ASSERT_EQ(stack_size(s), 1);
    stack_destroy(s);
}

TEST(test_empty_pop) {
    Stack *s = stack_create(4);
    // Popping from empty stack should return -1, not crash
    ASSERT_EQ(stack_pop(s), -1);
    ASSERT_EQ(stack_size(s), 0);
    stack_destroy(s);
}

TEST(test_growth) {
    Stack *s = stack_create(2);
    // Push more than initial capacity to trigger realloc;
    // growth must be reported as success.
    for (int i = 0; i < 20; i++) {
        ASSERT_EQ(stack_push(s, i * 10), 0);
    }
    ASSERT_EQ(stack_size(s), 20);
    ASSERT_EQ(stack_pop(s), 190);
    ASSERT_EQ(stack_pop(s), 180);
    stack_destroy(s);
}

/* Note: the failure branch of the contract (realloc returning NULL,
 * push returning -1 with the stack untouched) is NOT exercised by
 * these tests — forcing an allocation failure deterministically would
 * need an injectable allocator. The contract still requires it. */

int main(void) {
    RUN_TEST(test_create_and_destroy);
    RUN_TEST(test_push_pop);
    RUN_TEST(test_peek);
    RUN_TEST(test_empty_pop);
    RUN_TEST(test_growth);
    TEST_REPORT();
}
#endif
