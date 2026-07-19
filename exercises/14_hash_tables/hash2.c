// hash2.c - Open addressing: the probe must go on
//
// An open-addressing table stores entries directly in the array: the
// hash picks the home slot, and on a collision the entry moves to the
// NEXT free slot (linear probing). That one word — "next" — carries
// two rules this file breaks:
//   1. the probe WRAPS: after the last slot comes slot 0. This probe
//      stops at the end of the array, so any cluster that reaches the
//      last slot loses entries;
//   2. probing applies to LOOKUP too: get() here checks only the home
//      slot, so any key that was displaced by a collision is
//      "not found" even though it is right there, one slot over.
//
// (Editorial note: a fixed-size probing table is a routing table, a
// device registry, an in-memory symbol index — any small map without
// malloc. Interpreters keep globals in one, but nothing here needs
// an interpreter.)
//
// The tests use keys with known colliding hashes (computed offline):
// "size"/"time" share home slot 4; "port"/"path"/"name" share home
// slot 6 — a cluster that must wrap past the end of the 8-slot table.

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

#define TABLE_CAPACITY 8

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
    const char *key; /* NULL-able; not owned */
    int value;
    bool used;
} Entry;

typedef struct {
    Entry entries[TABLE_CAPACITY];
} Table;

void table_init(Table *t) {
    memset(t, 0, sizeof(*t));
}

// Inserts or updates. Returns 0, or -1 if the table is full.
int table_put(Table *t, const char *key, int value) {
    uint32_t start = fnv1a(key) % TABLE_CAPACITY;
    // BUG: the probe stops at the end of the array instead of
    // wrapping around to slot 0.
    for (uint32_t i = start; i < TABLE_CAPACITY; i++) {
        Entry *e = &t->entries[i];
        if (!e->used) {
            e->key = key;
            e->value = value;
            e->used = true;
            return 0;
        }
        if (strcmp(e->key, key) == 0) {
            e->value = value;
            return 0;
        }
    }
    return -1;
}

// Returns 0 and stores the value, or -1 if the key is absent
// (*out untouched in that case).
int table_get(const Table *t, const char *key, int *out) {
    const Entry *e = &t->entries[fnv1a(key) % TABLE_CAPACITY];
    // BUG: no probing at all — a key displaced by a collision is
    // reported missing even though it sits one slot over.
    if (e->used && strcmp(e->key, key) == 0) {
        *out = e->value;
        return 0;
    }
    return -1;
}

#ifndef TEST
int main(void) {
    Table t;
    table_init(&t);
    table_put(&t, "port", 8080);
    table_put(&t, "path", 1);  /* collides with "port": home slot 6 */
    table_put(&t, "name", 2);  /* same cluster: must wrap past slot 7 */

    int v = 0;
    const char *keys[] = {"port", "path", "name"};
    for (int i = 0; i < 3; i++) {
        if (table_get(&t, keys[i], &v) == 0) {
            printf("%s = %d\n", keys[i], v);
        } else {
            printf("%s = NOT FOUND (bad!)\n", keys[i]);
        }
    }
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_put_and_get) {
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "cpu", 4), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "cpu", &v), 0);
    ASSERT_EQ(v, 4);
}

TEST(test_update_existing_key) {
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "cpu", 4), 0);
    ASSERT_EQ(table_put(&t, "cpu", 8), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "cpu", &v), 0);
    ASSERT_EQ(v, 8);
}

TEST(test_colliding_keys_both_found) {
    // "size" and "time" share home slot 4: the second is displaced,
    // and get() must PROBE to find it.
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "size", 1), 0);
    ASSERT_EQ(table_put(&t, "time", 2), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "size", &v), 0);
    ASSERT_EQ(v, 1);
    ASSERT_EQ(table_get(&t, "time", &v), 0);
    ASSERT_EQ(v, 2);
}

TEST(test_probe_wraps_around) {
    // "port"/"path"/"name" all have home slot 6 of 8: the cluster
    // fills slots 6 and 7 and must WRAP to slot 0 for the third key.
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "port", 1), 0);
    ASSERT_EQ(table_put(&t, "path", 2), 0);
    ASSERT_EQ(table_put(&t, "name", 3), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "name", &v), 0);
    ASSERT_EQ(v, 3);
}

TEST(test_missing_key) {
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "size", 1), 0);
    int v = 99;
    ASSERT_EQ(table_get(&t, "nope", &v), -1);
    ASSERT_EQ(v, 99); /* untouched */
}

TEST(test_full_table_rejects) {
    Table t;
    table_init(&t);
    const char *keys[TABLE_CAPACITY] = {"a", "b", "c", "d", "e", "f", "g", "h"};
    for (int i = 0; i < TABLE_CAPACITY; i++) {
        ASSERT_EQ(table_put(&t, keys[i], i), 0);
    }
    ASSERT_EQ(table_put(&t, "overflow", 9), -1);
    /* everything already inserted is still reachable */
    int v = 0;
    for (int i = 0; i < TABLE_CAPACITY; i++) {
        ASSERT_EQ(table_get(&t, keys[i], &v), 0);
        ASSERT_EQ(v, i);
    }
}

int main(void) {
    RUN_TEST(test_put_and_get);
    RUN_TEST(test_update_existing_key);
    RUN_TEST(test_colliding_keys_both_found);
    RUN_TEST(test_probe_wraps_around);
    RUN_TEST(test_missing_key);
    RUN_TEST(test_full_table_rejects);
    TEST_REPORT();
}
#endif
