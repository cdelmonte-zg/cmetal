// arena1.c - The bump allocator: lifetime by construction
//
// An arena hands out memory by BUMPING a cursor through one big block:
// allocation is an addition, and freeing is resetting the cursor —
// everything allocated in the arena dies together. That is the whole
// idea: tie many small lifetimes to one big one.
//
// Two rules carry the allocator, and this one breaks both:
//   1. ALIGNMENT: the cursor must be rounded up before every
//      allocation, or an odd-sized request leaves the next pointer
//      misaligned — storing a double through it is undefined behavior
//      that UBSan reports on the spot (the demo does exactly that);
//   2. CAPACITY: the bump must stop at the end of the block. This one
//      just keeps bumping — the demo walks straight off the end and
//      AddressSanitizer names the overflow.
//
// Contract: arena_alloc returns a pointer aligned for any object type,
// or NULL when the request doesn't fit (the arena stays usable).
// Watch the capacity check itself: `used + size > capacity` can
// overflow — compare in the other direction (encodings2's lesson).
//
// (Editorial note: per-request scratch space is any server handler,
// parser pass or frame of a game loop. Compilers arena their ASTs,
// but nothing here requires one.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

typedef struct {
    uint8_t *block;
    size_t capacity;
    size_t used;
} Arena;

// The strictest alignment any C object needs.
#define ARENA_ALIGN _Alignof(max_align_t)

// Returns 0, or -1 if the backing allocation fails.
int arena_init(Arena *a, size_t capacity) {
    a->block = CMETAL_MALLOC(capacity);
    if (!a->block) {
        return -1;
    }
    a->capacity = capacity;
    a->used = 0;
    return 0;
}

// Returns a suitably aligned pointer to `size` fresh bytes, or NULL
// when the request doesn't fit. The arena stays usable either way.
void *arena_alloc(Arena *a, size_t size) {
    // BUG: no alignment — after a 1-byte allocation the cursor is odd,
    // and the next caller gets a misaligned pointer.
    // BUG: no capacity check — the cursor happily bumps past the end
    // of the block.
    void *p = a->block + a->used;
    a->used += size;
    return p;
}

// Everything allocated so far dies at once; the block is reused.
void arena_reset(Arena *a) {
    a->used = 0;
}

void arena_destroy(Arena *a) {
    free(a->block);
    a->block = NULL;
    a->capacity = 0;
    a->used = 0;
}

#ifndef TEST
int main(void) {
    Arena a;
    if (arena_init(&a, 64) != 0) {
        return 1;
    }

    // One odd-sized allocation, then a double: the second pointer must
    // still be aligned — UBSan checks the store.
    char *tag = arena_alloc(&a, 1);
    *tag = 'x';
    double *d = arena_alloc(&a, sizeof(double));
    *d = 3.14;
    printf("tag=%c d=%g\n", *tag, *d);

    // Exhaustion: this request cannot fit a 64-byte arena; a correct
    // arena says NULL, this one walks off the block.
    char *big = arena_alloc(&a, 128);
    if (big == NULL) {
        printf("128 bytes: refused (correct)\n");
    } else {
        memset(big, 0, 128); /* ASan has opinions about this */
        printf("128 bytes: accepted (bad!)\n");
    }

    arena_destroy(&a);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_allocations_are_aligned) {
    Arena a;
    ASSERT_EQ(arena_init(&a, 256), 0);
    /* odd-sized requests must not poison the next allocation */
    for (int i = 0; i < 5; i++) {
        void *p = arena_alloc(&a, 3);
        ASSERT(p != NULL);
        ASSERT_EQ((uintptr_t)p % ARENA_ALIGN, 0u);
    }
    arena_destroy(&a);
}

TEST(test_exhaustion_returns_null) {
    Arena a;
    ASSERT_EQ(arena_init(&a, 64), 0);
    ASSERT(arena_alloc(&a, 128) == NULL); /* larger than the arena */
    void *p = arena_alloc(&a, 32);
    ASSERT(p != NULL); /* still usable after a refused request */
    ASSERT(arena_alloc(&a, 64) == NULL); /* larger than what's left */
    arena_destroy(&a);
}

TEST(test_reset_recycles_everything) {
    Arena a;
    ASSERT_EQ(arena_init(&a, 64), 0);
    ASSERT(arena_alloc(&a, 48) != NULL);
    ASSERT(arena_alloc(&a, 48) == NULL); /* full */
    arena_reset(&a);
    ASSERT(arena_alloc(&a, 48) != NULL); /* whole block back */
    arena_destroy(&a);
}

TEST(test_absurd_request_cannot_wrap) {
    // The naive check `used + size > capacity` wraps for huge sizes
    // and silently accepts them — the overflow-safe form cannot.
    Arena a;
    ASSERT_EQ(arena_init(&a, 64), 0);
    ASSERT(arena_alloc(&a, 16) != NULL);
    ASSERT(arena_alloc(&a, SIZE_MAX) == NULL);
    arena_destroy(&a);
}

TEST(test_init_failure_is_reported) {
    Arena a;
    cmetal_fail_next_alloc();
    ASSERT_EQ(arena_init(&a, 64), -1);
}

int main(void) {
    RUN_TEST(test_allocations_are_aligned);
    RUN_TEST(test_exhaustion_returns_null);
    RUN_TEST(test_reset_recycles_everything);
    RUN_TEST(test_absurd_request_cannot_wrap);
    RUN_TEST(test_init_failure_is_reported);
    TEST_REPORT();
}
#endif
