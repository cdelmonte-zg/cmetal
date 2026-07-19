// arena3.c - Escape discipline: what outlives the arena leaves it
//
// An arena makes lifetimes trivial INSIDE its scope — and treacherous
// at the boundary. Anything that must survive the arena (a response,
// a result, an error message) has to be COPIED OUT to storage with
// the longer lifetime; returning an arena pointer hands the caller
// memory that dies with the next destroy.
//
// The scenario: a request handler parses "name=value" using the
// request's scratch arena and returns the VALUE. The request ends,
// the arena is destroyed — and the caller still holds the result.
//
// Bugs:
//   1. the returned value lives in the scratch arena: after
//      arena_destroy the caller reads freed memory (the demo does —
//      ASan calls it heap-use-after-free). The tests also check it
//      structurally: the result must NOT be arena-owned;
//   2. a line without '=' walks past the end of the buffer instead of
//      being rejected (return NULL, the error path of any parser).
//
// Contract: handle_request returns a malloc'd string THE CALLER OWNS
// (and frees), or NULL on parse error or allocation failure.
//
// (Editorial note: request/response boundaries exist in every server,
// callback API and pipeline stage. Interpreters hit this between
// compile-time and runtime lifetimes, but nothing here requires one.)

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

typedef struct {
    uint8_t *block;
    size_t capacity;
    size_t used;
} Arena;

#define ARENA_ALIGN _Alignof(max_align_t)

int arena_init(Arena *a, size_t capacity) {
    a->block = CMETAL_MALLOC(capacity);
    if (!a->block) {
        return -1;
    }
    a->capacity = capacity;
    a->used = 0;
    return 0;
}

void *arena_alloc(Arena *a, size_t size) {
    size_t start = (a->used + (ARENA_ALIGN - 1)) & ~(ARENA_ALIGN - 1);
    if (size > a->capacity || start > a->capacity - size) {
        return NULL;
    }
    void *p = a->block + start;
    a->used = start + size;
    return p;
}

void arena_destroy(Arena *a) {
    free(a->block);
    a->block = NULL;
    a->capacity = 0;
    a->used = 0;
}

// True if p points into the arena's block. (Pointers are compared as
// integers: uintptr_t round-trips are implementation-defined but
// reliable on every platform cmetal targets — and this check is a
// teaching aid, not part of the data path.)
bool arena_owns(const Arena *a, const void *p) {
    uintptr_t lo = (uintptr_t)a->block;
    uintptr_t x = (uintptr_t)p;
    return a->block != NULL && x >= lo && x < lo + a->capacity;
}

// Copies s into the arena (scratch lifetime). NULL if it doesn't fit.
static char *arena_strdup(Arena *a, const char *s, size_t len) {
    char *copy = arena_alloc(a, len + 1);
    if (!copy) {
        return NULL;
    }
    memcpy(copy, s, len);
    copy[len] = '\0';
    return copy;
}

// Parses "name=value" using `scratch` for temporaries and returns the
// value as a string THE CALLER OWNS (malloc'd; caller frees).
// Returns NULL on parse error or allocation failure.
char *handle_request(Arena *scratch, const char *line) {
    // BUG: if there is no '=', this scans right past the terminator.
    const char *eq = line;
    while (*eq != '=') {
        eq++;
    }

    size_t value_len = strlen(eq + 1);
    // BUG: the value is placed in the SCRATCH arena — it dies with the
    // request, but it is exactly the part that must outlive it.
    char *value = arena_strdup(scratch, eq + 1, value_len);
    return value;
}

#ifndef TEST
int main(void) {
    Arena scratch;
    if (arena_init(&scratch, 256) != 0) {
        return 1;
    }

    char *value = handle_request(&scratch, "user=admin");
    if (!value) {
        arena_destroy(&scratch);
        return 1;
    }

    // The request is over: its scratch memory goes away...
    arena_destroy(&scratch);

    // ...and the caller still needs the result. If it lived in the
    // arena, this read is use-after-free — ASan will say so.
    printf("value: %s\n", value);
    free(value);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_value_is_extracted) {
    Arena scratch;
    ASSERT_EQ(arena_init(&scratch, 256), 0);
    char *value = handle_request(&scratch, "user=admin");
    ASSERT(value != NULL);
    ASSERT_STR_EQ(value, "admin");
    free(value);
    arena_destroy(&scratch);
}

TEST(test_result_escapes_the_arena) {
    // The structural check: the returned value must NOT live in the
    // scratch arena, or it dies with the request.
    Arena scratch;
    ASSERT_EQ(arena_init(&scratch, 256), 0);
    char *value = handle_request(&scratch, "key=payload");
    ASSERT(value != NULL);
    ASSERT(!arena_owns(&scratch, value));
    arena_destroy(&scratch);
    ASSERT_STR_EQ(value, "payload"); /* alive after the arena is gone */
    free(value);
}

TEST(test_missing_equals_is_rejected) {
    Arena scratch;
    ASSERT_EQ(arena_init(&scratch, 256), 0);
    ASSERT(handle_request(&scratch, "no delimiter here") == NULL);
    ASSERT(handle_request(&scratch, "") == NULL);
    arena_destroy(&scratch);
}

TEST(test_empty_value_is_valid) {
    Arena scratch;
    ASSERT_EQ(arena_init(&scratch, 256), 0);
    char *value = handle_request(&scratch, "key=");
    ASSERT(value != NULL);
    ASSERT_STR_EQ(value, "");
    free(value);
    arena_destroy(&scratch);
}

TEST(test_allocation_failure_returns_null) {
    Arena scratch;
    ASSERT_EQ(arena_init(&scratch, 256), 0);
    cmetal_fail_next_alloc();
    ASSERT(handle_request(&scratch, "key=value") == NULL);
    arena_destroy(&scratch);
}

int main(void) {
    RUN_TEST(test_value_is_extracted);
    RUN_TEST(test_result_escapes_the_arena);
    RUN_TEST(test_missing_equals_is_rejected);
    RUN_TEST(test_empty_value_is_valid);
    RUN_TEST(test_allocation_failure_returns_null);
    TEST_REPORT();
}
#endif
