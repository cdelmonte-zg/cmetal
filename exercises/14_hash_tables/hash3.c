// hash3.c - Deletion breaks probe chains: tombstones
//
// In an open-addressing table, colliding entries form a probe CHAIN:
// lookup walks from the home slot until it finds the key or an EMPTY
// slot. Deletion is where naive implementations die: clearing a slot
// back to EMPTY cuts every chain that passed through it — keys
// inserted after the deleted one become unreachable while still
// sitting in the table.
//
// The fix is a third slot state, the TOMBSTONE: "someone lived here".
// Lookup walks THROUGH tombstones (the chain continues); insertion may
// REUSE them (the space is free). Both halves matter: without reuse,
// a busy table slowly fills with ghosts until no insert succeeds.
//
// (Editorial note: any cache, session table or connection registry
// with removal has this exact problem — it is not an interpreter
// topic, though interpreters' global tables hit it too.)
//
// TODO: add a SLOT_TOMBSTONE state, make table_delete use it, make
// get probe through it, and make put reuse it. Colliding keys for the
// tests (computed offline): "size"/"time"/"val" share home slot 4.

#include <stdio.h>
#include <stdint.h>
#include <string.h>

#define TABLE_CAPACITY 8

#define SLOT_EMPTY 0
#define SLOT_USED 1
/* TODO: a third state is missing. */

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
    const char *key; /* not owned */
    int value;
    int state;
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
    for (uint32_t n = 0; n < TABLE_CAPACITY; n++) {
        Entry *e = &t->entries[(start + n) % TABLE_CAPACITY];
        if (e->state != SLOT_USED) {
            e->key = key;
            e->value = value;
            e->state = SLOT_USED;
            return 0;
        }
        if (strcmp(e->key, key) == 0) {
            e->value = value;
            return 0;
        }
    }
    return -1;
}

// Returns 0 and stores the value, or -1 if absent (*out untouched).
int table_get(const Table *t, const char *key, int *out) {
    uint32_t start = fnv1a(key) % TABLE_CAPACITY;
    for (uint32_t n = 0; n < TABLE_CAPACITY; n++) {
        const Entry *e = &t->entries[(start + n) % TABLE_CAPACITY];
        if (e->state == SLOT_EMPTY) {
            return -1; /* end of the probe chain */
        }
        if (e->state == SLOT_USED && strcmp(e->key, key) == 0) {
            *out = e->value;
            return 0;
        }
    }
    return -1;
}

// Removes the key. Returns 0, or -1 if absent.
int table_delete(Table *t, const char *key) {
    uint32_t start = fnv1a(key) % TABLE_CAPACITY;
    for (uint32_t n = 0; n < TABLE_CAPACITY; n++) {
        Entry *e = &t->entries[(start + n) % TABLE_CAPACITY];
        if (e->state == SLOT_EMPTY) {
            return -1;
        }
        if (e->state == SLOT_USED && strcmp(e->key, key) == 0) {
            // BUG: clearing back to EMPTY cuts every probe chain that
            // ran through this slot — keys inserted past it are lost.
            e->state = SLOT_EMPTY;
            e->key = NULL;
            return 0;
        }
    }
    return -1;
}

#ifndef TEST
int main(void) {
    Table t;
    table_init(&t);
    table_put(&t, "size", 1);
    table_put(&t, "time", 2); /* same home slot as "size": chained */

    table_delete(&t, "size");

    int v = 0;
    if (table_get(&t, "time", &v) == 0) {
        printf("time = %d\n", v);
    } else {
        printf("time = LOST after deleting its chain predecessor (bad!)\n");
    }
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_delete_then_miss) {
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "cpu", 1), 0);
    ASSERT_EQ(table_delete(&t, "cpu"), 0);
    int v = 99;
    ASSERT_EQ(table_get(&t, "cpu", &v), -1);
    ASSERT_EQ(v, 99);
    ASSERT_EQ(table_delete(&t, "cpu"), -1); /* already gone */
}

TEST(test_chain_survives_deletion) {
    // "size" and "time" share home slot 4; deleting "size" must NOT
    // make "time" unreachable.
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "size", 1), 0);
    ASSERT_EQ(table_put(&t, "time", 2), 0);
    ASSERT_EQ(table_delete(&t, "size"), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "time", &v), 0);
    ASSERT_EQ(v, 2);
}

TEST(test_slot_is_reusable_after_delete) {
    Table t;
    table_init(&t);
    ASSERT_EQ(table_put(&t, "size", 1), 0);
    ASSERT_EQ(table_put(&t, "time", 2), 0);
    ASSERT_EQ(table_delete(&t, "size"), 0);
    /* "val" shares the same home slot: it may take the freed spot */
    ASSERT_EQ(table_put(&t, "val", 3), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "val", &v), 0);
    ASSERT_EQ(v, 3);
    ASSERT_EQ(table_get(&t, "time", &v), 0);
    ASSERT_EQ(v, 2);
}

TEST(test_ghosts_do_not_fill_the_table) {
    // A busy table: insert and delete over and over. Without tombstone
    // REUSE, the ghosts eat every slot and inserts start failing even
    // though the table is almost empty.
    Table t;
    table_init(&t);
    for (int i = 0; i < 20; i++) {
        ASSERT_EQ(table_put(&t, "key", i), 0);
        ASSERT_EQ(table_delete(&t, "key"), 0);
    }
    ASSERT_EQ(table_put(&t, "size", 7), 0);
    int v = 0;
    ASSERT_EQ(table_get(&t, "size", &v), 0);
    ASSERT_EQ(v, 7);
}

int main(void) {
    RUN_TEST(test_delete_then_miss);
    RUN_TEST(test_chain_survives_deletion);
    RUN_TEST(test_slot_is_reusable_after_delete);
    RUN_TEST(test_ghosts_do_not_fill_the_table);
    TEST_REPORT();
}
#endif
