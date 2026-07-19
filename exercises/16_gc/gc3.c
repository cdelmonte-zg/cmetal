// gc3.c - Finalization: collected objects own things too
//
// Objects rarely consist of just themselves: they own payloads —
// strings, buffers, handles. When the collector frees an object, it
// must release what the object OWNS, and nothing it merely borrows.
// That makes ownership a per-field design decision, and this heap
// gets it wrong twice:
//   1. the sweep frees objects but never their `name` payload — every
//      collected object leaks its string (the leak sanitizer counts
//      them at demo exit);
//   2. obj_share_name hands the SAME buffer to a second object — two
//      owners, one allocation, and the sweep frees it twice. Sharing
//      contents means duplicating them (or not owning them at all —
//      but pick ONE rule and encode it).
//
// Contract: every object owns its name exclusively. set_name and
// share_name return 0, or -1 if allocation fails, leaving the object
// unchanged (allocations go through CMETAL_MALLOC).
//
// (Editorial note: resource-owning records live in every registry,
// cache and connection table; "free the container, forget the
// contents" is a leak generator everywhere. Interpreters finalize
// heap objects the same way, but nothing here requires one.)

#include <stdio.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

#define MAX_REFS 4
#define MAX_ROOTS 8

typedef struct Obj Obj;
struct Obj {
    Obj *next;
    bool marked;
    Obj *refs[MAX_REFS];
    size_t ref_count;
    char *name; /* owned exclusively by this object, or NULL */
};

typedef struct {
    Obj *all;
    Obj *roots[MAX_ROOTS];
    size_t root_count;
    size_t live;
} Heap;

void heap_init(Heap *h) {
    h->all = NULL;
    h->root_count = 0;
    h->live = 0;
}

Obj *heap_new(Heap *h) {
    Obj *o = CMETAL_MALLOC(sizeof(Obj));
    if (!o) {
        return NULL;
    }
    o->marked = false;
    o->ref_count = 0;
    o->name = NULL;
    o->next = h->all;
    h->all = o;
    h->live++;
    return o;
}

int heap_add_root(Heap *h, Obj *o) {
    if (h->root_count == MAX_ROOTS) {
        return -1;
    }
    h->roots[h->root_count++] = o;
    return 0;
}

void heap_remove_root(Heap *h, Obj *o) {
    for (size_t i = 0; i < h->root_count; i++) {
        if (h->roots[i] == o) {
            h->roots[i] = h->roots[--h->root_count];
            return;
        }
    }
}

int obj_add_ref(Obj *from, Obj *to) {
    if (from->ref_count == MAX_REFS) {
        return -1;
    }
    from->refs[from->ref_count++] = to;
    return 0;
}

// Gives o its own copy of name. Returns 0, or -1 on allocation
// failure (o unchanged).
int obj_set_name(Obj *o, const char *name) {
    size_t len = strlen(name);
    char *copy = CMETAL_MALLOC(len + 1);
    if (!copy) {
        return -1;
    }
    memcpy(copy, name, len + 1);
    free(o->name); /* release the previous name, if any */
    o->name = copy;
    return 0;
}

// Makes dst carry the same name as src.
// Returns 0, or -1 on allocation failure (dst unchanged).
int obj_share_name(Obj *dst, const Obj *src) {
    // BUG: this hands dst the SAME buffer src owns — two owners, one
    // allocation, and the sweep will free it twice.
    free(dst->name);
    dst->name = src->name;
    return 0;
}

static void mark(Obj *o) {
    if (o->marked) {
        return;
    }
    o->marked = true;
    for (size_t i = 0; i < o->ref_count; i++) {
        mark(o->refs[i]);
    }
}

void gc_collect(Heap *h) {
    for (Obj *o = h->all; o; o = o->next) {
        o->marked = false;
    }
    for (size_t i = 0; i < h->root_count; i++) {
        mark(h->roots[i]);
    }
    Obj **link = &h->all;
    while (*link) {
        Obj *o = *link;
        if (!o->marked) {
            *link = o->next;
            // BUG: the object owns its name — freeing the container
            // and forgetting the contents leaks one string per
            // collected object.
            free(o);
            h->live--;
        } else {
            link = &o->next;
        }
    }
}

void heap_destroy(Heap *h) {
    Obj *o = h->all;
    while (o) {
        Obj *next = o->next;
        free(o->name);
        free(o);
        o = next;
    }
    heap_init(h);
}

#ifndef TEST
int main(void) {
    Heap h;
    heap_init(&h);

    // Named garbage: the objects get collected, and their names must
    // go with them — the leak sanitizer audits the exit.
    for (int i = 0; i < 3; i++) {
        Obj *o = heap_new(&h);
        if (!o || obj_set_name(o, "temporary") != 0) {
            heap_destroy(&h);
            return 1;
        }
    }
    gc_collect(&h);
    printf("live after collect: %zu (expected 0)\n", h.live);

    heap_destroy(&h);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_set_name_owns_a_copy) {
    Heap h;
    heap_init(&h);
    Obj *o = heap_new(&h);
    ASSERT(o != NULL);
    char buf[16];
    snprintf(buf, sizeof(buf), "worker");
    ASSERT_EQ(obj_set_name(o, buf), 0);
    snprintf(buf, sizeof(buf), "gone");
    ASSERT_STR_EQ(o->name, "worker");
    heap_destroy(&h);
}

TEST(test_share_duplicates_the_name) {
    // Exclusive ownership: after sharing, the two objects hold equal
    // CONTENTS in distinct buffers.
    Heap h;
    heap_init(&h);
    Obj *a = heap_new(&h);
    Obj *b = heap_new(&h);
    ASSERT(a != NULL);
    ASSERT(b != NULL);
    ASSERT_EQ(obj_set_name(a, "shared"), 0);
    ASSERT_EQ(obj_share_name(b, a), 0);
    ASSERT_STR_EQ(b->name, "shared");
    ASSERT(a->name != b->name);
    heap_destroy(&h);
}

TEST(test_collected_objects_release_their_names) {
    Heap h;
    heap_init(&h);
    Obj *o = heap_new(&h);
    ASSERT(o != NULL);
    ASSERT_EQ(obj_set_name(o, "doomed"), 0);
    gc_collect(&h); /* no roots: o and its name both go */
    ASSERT_EQ(h.live, 0u);
    heap_destroy(&h);
}

TEST(test_set_name_failure_leaves_object_unchanged) {
    Heap h;
    heap_init(&h);
    Obj *o = heap_new(&h);
    ASSERT(o != NULL);
    ASSERT_EQ(obj_set_name(o, "keep"), 0);
    cmetal_fail_next_alloc();
    ASSERT_EQ(obj_set_name(o, "never"), -1);
    ASSERT_STR_EQ(o->name, "keep");
    cmetal_fail_next_alloc();
    Obj *b = heap_new(&h);
    ASSERT(b == NULL);
    heap_destroy(&h);
}

int main(void) {
    RUN_TEST(test_set_name_owns_a_copy);
    RUN_TEST(test_share_duplicates_the_name);
    RUN_TEST(test_collected_objects_release_their_names);
    RUN_TEST(test_set_name_failure_leaves_object_unchanged);
    TEST_REPORT();
}
#endif
