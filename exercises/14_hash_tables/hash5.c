// hash5.c - String interning: one canonical copy of every string
//
// Interning gives each distinct string ONE canonical, pool-owned copy:
// after interning, comparing identities (==) IS comparing contents —
// function_pointers3 taught that == on strings is a bug; interning is
// the discipline that makes it correct, and it is why interned lookups
// are O(1) pointer compares.
//
// Two rules carry the whole idea, and this pool breaks both:
//   1. the pool must OWN its strings: storing the caller's pointer
//      means the "canonical" string dies (or mutates) with the
//      caller's buffer;
//   2. interning the same contents twice must return the SAME pointer
//      — that identity guarantee is the entire point. This pool just
//      appends, so equal strings get different copies.
//
// Contract: pool_intern returns the canonical pointer, or NULL if
// allocation fails (pool untouched). pool_free releases everything.
//
// (Editorial note: symbol tables, config keys, log field names,
// protocol atoms — any system that repeats the same small strings
// benefits. Interpreters intern identifiers, but nothing here
// requires one.)

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cmetal_alloc.h"

typedef struct {
    char **items; /* owned canonical strings */
    size_t count;
    size_t capacity;
} Pool;

void pool_init(Pool *p) {
    p->items = NULL;
    p->count = 0;
    p->capacity = 0;
}

void pool_free(Pool *p) {
    for (size_t i = 0; i < p->count; i++) {
        free(p->items[i]);
    }
    free(p->items);
    pool_init(p);
}

// Returns the canonical pointer for the contents of s, or NULL if
// allocation fails (pool untouched).
const char *pool_intern(Pool *p, const char *s) {
    // BUG: no lookup — equal contents interned twice get two
    // different "canonical" pointers, which defeats the whole idea.
    if (p->count == p->capacity) {
        size_t new_cap = p->capacity == 0 ? 8 : p->capacity * 2;
        char **fresh = CMETAL_REALLOC(p->items, new_cap * sizeof(char *));
        if (!fresh) {
            return NULL;
        }
        p->items = fresh;
        p->capacity = new_cap;
    }
    // BUG: stores the CALLER's pointer — the pool doesn't own a copy,
    // so the "canonical" string lives and dies with the caller's
    // buffer (pool_free will also free memory it never allocated).
    p->items[p->count] = (char *)s;
    p->count++;
    return p->items[p->count - 1];
}

#ifndef TEST
int main(void) {
    Pool pool;
    pool_init(&pool);

    // Interning from a scratch buffer: the pool must survive the
    // buffer being reused.
    char buf[16];
    snprintf(buf, sizeof(buf), "port");
    const char *canonical = pool_intern(&pool, buf);
    snprintf(buf, sizeof(buf), "path"); /* buffer reused */

    const char *again = pool_intern(&pool, "port");
    printf("canonical: \"%s\"  again: \"%s\"  same pointer: %s\n",
           canonical ? canonical : "(null)", again ? again : "(null)",
           canonical == again ? "yes" : "NO (bad!)");

    pool_free(&pool);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_same_contents_same_pointer) {
    Pool p;
    pool_init(&p);
    const char *a = pool_intern(&p, "port");
    const char *b = pool_intern(&p, "port");
    ASSERT(a != NULL);
    ASSERT(a == b); /* identity IS the contract */
    ASSERT_STR_EQ(a, "port");
    pool_free(&p);
}

TEST(test_distinct_contents_distinct_pointers) {
    Pool p;
    pool_init(&p);
    const char *a = pool_intern(&p, "port");
    const char *b = pool_intern(&p, "path");
    ASSERT(a != NULL);
    ASSERT(b != NULL);
    ASSERT(a != b);
    ASSERT_STR_EQ(a, "port");
    ASSERT_STR_EQ(b, "path");
    pool_free(&p);
}

TEST(test_pool_owns_its_strings) {
    // The caller's buffer is reused after interning: the canonical
    // string must not change with it.
    Pool p;
    pool_init(&p);
    char buf[16];
    snprintf(buf, sizeof(buf), "user");
    const char *canonical = pool_intern(&p, buf);
    ASSERT(canonical != NULL);
    snprintf(buf, sizeof(buf), "gone");
    ASSERT_STR_EQ(canonical, "user");
    pool_free(&p);
}

TEST(test_failed_allocation_leaves_pool_untouched) {
    Pool p;
    pool_init(&p);
    const char *a = pool_intern(&p, "port");
    ASSERT(a != NULL);
    cmetal_fail_next_alloc();
    ASSERT(pool_intern(&p, "brand-new") == NULL);
    /* the existing intern is intact and identity still holds */
    ASSERT(pool_intern(&p, "port") == a);
    pool_free(&p);
}

int main(void) {
    RUN_TEST(test_same_contents_same_pointer);
    RUN_TEST(test_distinct_contents_distinct_pointers);
    RUN_TEST(test_pool_owns_its_strings);
    RUN_TEST(test_failed_allocation_leaves_pool_untouched);
    TEST_REPORT();
}
#endif
