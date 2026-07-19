// hash4.c - Growth means REHASH, not copy
//
// A probing table degrades as it fills: past ~75% load, chains get
// long. The fix is growing the array — but an entry's home slot is
// `hash % capacity`, so when the capacity changes, ALMOST EVERY HOME
// MOVES. Growth must re-insert every entry into the new geometry.
// This implementation copies the old array byte-for-byte instead:
// entries stay at their old indices, lookups probe from the new home,
// and keys vanish without being gone.
//
// A second, quieter bug: the old array is never freed — every growth
// leaks the previous generation (the demo grows once and exits; the
// leak sanitizer does the counting).
//
// Contract: table_put returns 0, or -1 if allocation fails, leaving
// the table fully usable (allocation goes through CMETAL_MALLOC, so
// the tests can force that failure).
//
// (Editorial note: capacity-doubling with re-placement is any index
// that grows under load — connection maps, symbol indexes, dedup
// sets. Interpreters grow their tables the same way, but nothing
// here requires one.)
//
// Offline-computed detail the tests rely on: "size" has home slot 4
// in an 8-slot table but home slot 12 in a 16-slot one — after one
// growth, only a rehash finds it. "time" keeps home 4 in both.

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

#define FNV_OFFSET 2166136261u
#define FNV_PRIME 16777619u

static uint32_t fnv1a(const char *s) {
    uint32_t hash = FNV_OFFSET;
    for (const char *p = s; *p != '\0'; p++) {
        hash ^= (uint8_t)*p;
        hash *= FNV_PRIME;
    }
    return hash;
}

typedef struct {
    const char *key; /* not owned; NULL = empty */
    int value;
} Entry;

typedef struct {
    Entry *entries;
    size_t capacity;
    size_t count;
} Table;

#define INITIAL_CAPACITY 8

int table_init(Table *t) {
    t->entries = CMETAL_MALLOC(INITIAL_CAPACITY * sizeof(Entry));
    if (!t->entries) {
        return -1;
    }
    memset(t->entries, 0, INITIAL_CAPACITY * sizeof(Entry));
    t->capacity = INITIAL_CAPACITY;
    t->count = 0;
    return 0;
}

void table_free(Table *t) {
    free(t->entries);
    t->entries = NULL;
    t->capacity = 0;
    t->count = 0;
}

static int table_grow(Table *t) {
    size_t new_cap = t->capacity * 2;
    Entry *fresh = CMETAL_MALLOC(new_cap * sizeof(Entry));
    if (!fresh) {
        return -1; /* table untouched, caller informed */
    }
    memset(fresh, 0, new_cap * sizeof(Entry));
    // BUG: entries are copied to their OLD indices, but home slots are
    // `hash % capacity` and the capacity just changed — almost every
    // key now lives in the wrong place. Growth must RE-INSERT each
    // entry, probing in the new geometry.
    memcpy(fresh, t->entries, t->capacity * sizeof(Entry));
    // BUG: the previous array is never freed — one leak per growth.
    t->entries = fresh;
    t->capacity = new_cap;
    return 0;
}

// Inserts or updates. Returns 0, or -1 if allocation failed during
// growth (the table stays fully usable).
int table_put(Table *t, const char *key, int value) {
    if ((t->count + 1) * 4 > t->capacity * 3) {
        if (table_grow(t) != 0) {
            return -1;
        }
    }
    uint32_t start = fnv1a(key) % (uint32_t)t->capacity;
    for (size_t n = 0; n < t->capacity; n++) {
        Entry *e = &t->entries[(start + n) % t->capacity];
        if (e->key == NULL) {
            e->key = key;
            e->value = value;
            t->count++;
            return 0;
        }
        if (strcmp(e->key, key) == 0) {
            e->value = value;
            return 0;
        }
    }
    return -1; /* unreachable while growth keeps load below 100% */
}

// Returns 0 and stores the value, or -1 if absent (*out untouched).
int table_get(const Table *t, const char *key, int *out) {
    uint32_t start = fnv1a(key) % (uint32_t)t->capacity;
    for (size_t n = 0; n < t->capacity; n++) {
        const Entry *e = &t->entries[(start + n) % t->capacity];
        if (e->key == NULL) {
            return -1;
        }
        if (strcmp(e->key, key) == 0) {
            *out = e->value;
            return 0;
        }
    }
    return -1;
}

#ifndef TEST
int main(void) {
    Table t;
    if (table_init(&t) != 0) {
        return 1;
    }
    const char *keys[] = {"size", "time", "port", "path", "name", "user", "mode"};
    for (int i = 0; i < 7; i++) {
        if (table_put(&t, keys[i], i) != 0) {
            printf("put failed\n");
            table_free(&t);
            return 1;
        }
    }
    printf("capacity after 7 inserts: %zu\n", t.capacity);
    int v = 0;
    for (int i = 0; i < 7; i++) {
        if (table_get(&t, keys[i], &v) == 0) {
            printf("%s = %d\n", keys[i], v);
        } else {
            printf("%s = LOST in growth (bad!)\n", keys[i]);
        }
    }
    table_free(&t);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_put_get_before_growth) {
    Table t;
    ASSERT_EQ(table_init(&t), 0);
    ASSERT_EQ(table_put(&t, "size", 1), 0);
    ASSERT_EQ(table_put(&t, "time", 2), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "size", &v), 0);
    ASSERT_EQ(v, 1);
    table_free(&t);
}

TEST(test_growth_rehashes) {
    // 7 inserts push an 8-slot table past 75% load: it grows to 16.
    // "size" moves home (4 -> 12): only a real rehash finds it after.
    Table t;
    ASSERT_EQ(table_init(&t), 0);
    const char *keys[] = {"size", "time", "port", "path", "name", "user", "mode"};
    for (int i = 0; i < 7; i++) {
        ASSERT_EQ(table_put(&t, keys[i], i), 0);
    }
    ASSERT_EQ(t.capacity, 16u);
    int v = 0;
    for (int i = 0; i < 7; i++) {
        ASSERT_EQ(table_get(&t, keys[i], &v), 0);
        ASSERT_EQ(v, i);
    }
    table_free(&t);
}

TEST(test_failed_growth_leaves_table_usable) {
    Table t;
    ASSERT_EQ(table_init(&t), 0);
    const char *keys[] = {"size", "time", "port", "path", "name", "user"};
    for (int i = 0; i < 6; i++) {
        ASSERT_EQ(table_put(&t, keys[i], i), 0);
    }
    /* the 7th insert needs to grow: make that allocation fail */
    cmetal_fail_next_alloc();
    ASSERT_EQ(table_put(&t, "mode", 6), -1);
    ASSERT_EQ(t.capacity, 8u); /* untouched */
    int v = 0;
    for (int i = 0; i < 6; i++) {
        ASSERT_EQ(table_get(&t, keys[i], &v), 0);
        ASSERT_EQ(v, i);
    }
    /* and the table still works once memory is back */
    ASSERT_EQ(table_put(&t, "mode", 6), 0);
    ASSERT_EQ(table_get(&t, "mode", &v), 0);
    ASSERT_EQ(v, 6);
    table_free(&t);
}

int main(void) {
    RUN_TEST(test_put_get_before_growth);
    RUN_TEST(test_growth_rehashes);
    RUN_TEST(test_failed_growth_leaves_table_usable);
    TEST_REPORT();
}
#endif
