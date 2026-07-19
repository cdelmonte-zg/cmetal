// arena2.c - Growing arenas: chained blocks
//
// A fixed arena refuses work when it fills. A growing arena allocates
// a fresh block and CHAINS it to the previous ones — allocation stays
// a bump, and destroy walks the chain freeing every generation. (The
// block header uses a C11 flexible array member: header and payload
// in one allocation.)
//
// Two bugs and one missing contract:
//   1. when the arena grows, the fresh block REPLACES the chain head
//      without linking to it — every previous generation is orphaned,
//      and destroy frees only the newest block (the leak sanitizer
//      counts the rest at demo exit);
//   2. a request LARGER than a block slips through the growth check
//      and bumps straight past the fresh block's end (ASan);
//   3. growth can fail: arena_alloc must return NULL and leave the
//      arena usable (allocation goes through CMETAL_MALLOC, so the
//      tests force exactly that).
//
// Contract: requests up to ARENA_BLOCK_SIZE bytes are served (a fresh
// block is chained when needed); larger requests return NULL.
//
// (Editorial note: a parser or batch job whose total size is unknown
// up front is the natural user — read, grow, process, destroy.
// Compilers do per-phase pools this way, but nothing here needs one.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

#define ARENA_BLOCK_SIZE 64
#define ARENA_ALIGN _Alignof(max_align_t)

typedef struct Block {
    struct Block *next; /* older generation, or NULL */
    size_t used;
    uint8_t data[]; /* C11 flexible array member: the payload */
} Block;

typedef struct {
    Block *head; /* newest block; allocation happens here */
} Arena;

static Block *block_new(void) {
    Block *b = CMETAL_MALLOC(sizeof(Block) + ARENA_BLOCK_SIZE);
    if (!b) {
        return NULL;
    }
    b->next = NULL;
    b->used = 0;
    return b;
}

// Returns 0, or -1 if the first block cannot be allocated.
int arena_init(Arena *a) {
    a->head = block_new();
    return a->head ? 0 : -1;
}

// Returns an aligned pointer to `size` fresh bytes, growing the chain
// when the current block is full. Requests larger than
// ARENA_BLOCK_SIZE return NULL. On growth failure returns NULL and
// the arena stays usable.
void *arena_alloc(Arena *a, size_t size) {
    // BUG: a request larger than a whole block slips through the
    // check below (the fresh block won't fit it either) and bumps
    // past the end of the payload.
    size_t start = (a->head->used + (ARENA_ALIGN - 1)) & ~(ARENA_ALIGN - 1);
    if (start > ARENA_BLOCK_SIZE - size) {
        Block *fresh = block_new();
        if (!fresh) {
            return NULL;
        }
        // BUG: the fresh block replaces the head WITHOUT linking to
        // the previous generation — the old blocks are orphaned and
        // destroy will never find them.
        a->head = fresh;
        start = 0;
    }
    void *p = a->head->data + start;
    a->head->used = start + size;
    return p;
}

// Frees every generation.
void arena_destroy(Arena *a) {
    Block *b = a->head;
    while (b) {
        Block *next = b->next;
        free(b);
        b = next;
    }
    a->head = NULL;
}

#ifndef TEST
int main(void) {
    Arena a;
    if (arena_init(&a) != 0) {
        return 1;
    }

    // Enough allocations to force several generations: destroy must
    // free them ALL — the leak sanitizer audits this at exit.
    for (int i = 0; i < 10; i++) {
        char *p = arena_alloc(&a, 40);
        if (!p) {
            printf("alloc %d failed\n", i);
            arena_destroy(&a);
            return 1;
        }
        memset(p, i, 40);
    }
    printf("10 x 40 bytes across generations: ok\n");

    // An oversized request must be refused, not bumped past the block.
    char *big = arena_alloc(&a, ARENA_BLOCK_SIZE + 1);
    if (big == NULL) {
        printf("oversized request: refused (correct)\n");
    } else {
        memset(big, 0, ARENA_BLOCK_SIZE + 1); /* ASan will object */
        printf("oversized request: accepted (bad!)\n");
    }

    arena_destroy(&a);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_growth_serves_more_than_one_block) {
    Arena a;
    ASSERT_EQ(arena_init(&a), 0);
    /* 5 x 40 = 200 bytes through 64-byte blocks: must all succeed,
     * and every pointer stays aligned and writable */
    for (int i = 0; i < 5; i++) {
        char *p = arena_alloc(&a, 40);
        ASSERT(p != NULL);
        ASSERT_EQ((uintptr_t)p % ARENA_ALIGN, 0u);
        memset(p, i, 40);
    }
    arena_destroy(&a);
}

TEST(test_data_survives_growth) {
    Arena a;
    ASSERT_EQ(arena_init(&a), 0);
    char *first = arena_alloc(&a, 40);
    ASSERT(first != NULL);
    memset(first, 7, 40);
    /* force a new generation */
    ASSERT(arena_alloc(&a, 40) != NULL);
    /* the old generation's data must still be intact */
    for (int i = 0; i < 40; i++) {
        ASSERT_EQ(first[i], 7);
    }
    arena_destroy(&a);
}

TEST(test_oversized_request_refused) {
    Arena a;
    ASSERT_EQ(arena_init(&a), 0);
    ASSERT(arena_alloc(&a, ARENA_BLOCK_SIZE + 1) == NULL);
    ASSERT(arena_alloc(&a, 16) != NULL); /* still usable */
    arena_destroy(&a);
}

TEST(test_failed_growth_leaves_arena_usable) {
    Arena a;
    ASSERT_EQ(arena_init(&a), 0);
    ASSERT(arena_alloc(&a, 60) != NULL); /* nearly fills block one */
    cmetal_fail_next_alloc();
    ASSERT(arena_alloc(&a, 40) == NULL); /* growth fails cleanly */
    ASSERT(arena_alloc(&a, 40) != NULL); /* memory is back: grows */
    arena_destroy(&a);
}

int main(void) {
    RUN_TEST(test_growth_serves_more_than_one_block);
    RUN_TEST(test_data_survives_growth);
    RUN_TEST(test_oversized_request_refused);
    RUN_TEST(test_failed_growth_leaves_arena_usable);
    TEST_REPORT();
}
#endif
