// gc2.c - Sweep: free it AND forget it, then start clean
//
// After mark has flagged the reachable objects, sweep frees the rest.
// Two disciplines make it correct, and this sweep skips both:
//   1. freeing an object is not enough — it must also be UNLINKED
//      from the heap's object list. This sweep frees in place and
//      keeps walking a list that now threads through freed memory;
//      the very same loop reads o->next out of a freed object, and
//      the next collect (or destroy) walks the corpses again;
//   2. marks are per-collection state: they must be CLEARED before
//      each new mark phase. This collector never clears them, so an
//      object marked once survives every future collection — floating
//      garbage that can never be reclaimed, even after its last root
//      is gone.
//
// The unlink idiom worth learning: walk with a POINTER TO POINTER
// (Obj **link), so removing a node is one assignment and no special
// case for the head.
//
// (Editorial note: remove-while-iterating and stale per-pass state
// are list-processing bugs you'll meet in schedulers, event queues
// and connection pools — no interpreter required.)

#include <stdio.h>
#include <stdbool.h>
#include <stddef.h>
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
    int id;
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

Obj *heap_new(Heap *h, int id) {
    Obj *o = CMETAL_MALLOC(sizeof(Obj));
    if (!o) {
        return NULL;
    }
    o->marked = false;
    o->ref_count = 0;
    o->id = id;
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

// The mark phase is correct: visited-guard plus every reference.
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
    // BUG: marks from the previous collection are never cleared —
    // once marked, an object survives every future collection, even
    // long after its last root is gone.
    for (size_t i = 0; i < h->root_count; i++) {
        mark(h->roots[i]);
    }
    // BUG: freed objects stay LINKED in the all-list: this very loop
    // reads o->next out of freed memory, and so does everything that
    // walks the list afterwards.
    for (Obj *o = h->all; o; o = o->next) {
        if (!o->marked) {
            free(o);
            h->live--;
        }
    }
}

void heap_destroy(Heap *h) {
    Obj *o = h->all;
    while (o) {
        Obj *next = o->next;
        free(o);
        o = next;
    }
    heap_init(h);
}

#ifndef TEST
int main(void) {
    Heap h;
    heap_init(&h);

    Obj *root = heap_new(&h, 1);
    heap_add_root(&h, root);
    heap_new(&h, 2); /* garbage */
    heap_new(&h, 3); /* garbage */

    gc_collect(&h);
    printf("live after first collect: %zu (expected 1)\n", h.live);

    // A second collection walks the object list again: if the sweep
    // left freed objects linked, this reads them — ASan says where.
    gc_collect(&h);
    printf("live after second collect: %zu (expected 1)\n", h.live);

    heap_destroy(&h);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_collect_frees_garbage) {
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    heap_add_root(&h, root);
    heap_new(&h, 2);
    gc_collect(&h);
    ASSERT_EQ(h.live, 1u);
    heap_destroy(&h);
}

TEST(test_repeated_collections_are_safe) {
    // Sweep must leave a list that only contains live objects: the
    // second and third collections walk it again.
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    heap_add_root(&h, root);
    heap_new(&h, 2);
    heap_new(&h, 3);
    gc_collect(&h);
    gc_collect(&h);
    gc_collect(&h);
    ASSERT_EQ(h.live, 1u);
    heap_destroy(&h);
}

TEST(test_marks_do_not_leak_across_collections) {
    // Reachable today, garbage tomorrow: after the root goes away,
    // the NEXT collection must reclaim the object. A mark left over
    // from the previous pass would keep it alive forever.
    Heap h;
    heap_init(&h);
    Obj *a = heap_new(&h, 1);
    heap_add_root(&h, a);
    gc_collect(&h);
    ASSERT_EQ(h.live, 1u);
    heap_remove_root(&h, a);
    gc_collect(&h);
    ASSERT_EQ(h.live, 0u);
    heap_destroy(&h);
}

TEST(test_destroy_after_collect) {
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    heap_add_root(&h, root);
    heap_new(&h, 2);
    gc_collect(&h);
    /* destroy walks the same list the sweep just edited */
    heap_destroy(&h);
    ASSERT_EQ(h.live, 0u);
}

int main(void) {
    RUN_TEST(test_collect_frees_garbage);
    RUN_TEST(test_repeated_collections_are_safe);
    RUN_TEST(test_marks_do_not_leak_across_collections);
    RUN_TEST(test_destroy_after_collect);
    TEST_REPORT();
}
#endif
