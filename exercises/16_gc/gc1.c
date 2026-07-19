// gc1.c - Mark: reachability is transitive, and graphs have cycles
//
// When objects reference each other freely — caches with
// cross-references, documents with internal links, social graphs —
// ownership stops forming a tree and free() has no single owner to
// belong to. Tracing collection replaces the question "who frees
// this?" with "is this still REACHABLE from a root?".
//
// The mark phase answers it: start from the roots and flag everything
// you can walk to. Two properties carry it, and this mark breaks both:
//   1. reachability is TRANSITIVE over every reference — this mark
//      follows only the FIRST reference of each object, so a root's
//      second child is swept while still in use;
//   2. object graphs have CYCLES — this mark has no visited check, so
//      two objects referencing each other recurse until the stack
//      dies. The marked flag IS the visited check: it just has to be
//      honored.
//
// (Editorial note: mark is plain graph traversal with a visited set —
// the same walk you'd write for dependency resolution or reference
// auditing. Interpreters trace their heaps this way, but nothing here
// requires one.)

#include <stdio.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

#define MAX_REFS 4
#define MAX_ROOTS 8

typedef struct Obj Obj;
struct Obj {
    Obj *next; /* every allocated object, for the sweep */
    bool marked;
    Obj *refs[MAX_REFS];
    size_t ref_count;
    int id;
};

typedef struct {
    Obj *all;
    Obj *roots[MAX_ROOTS];
    size_t root_count;
    size_t live; /* maintained by heap_new and the sweep */
} Heap;

void heap_init(Heap *h) {
    h->all = NULL;
    h->root_count = 0;
    h->live = 0;
}

// Allocates an object into the heap. NULL if allocation fails.
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

// Registers a root. Returns 0, or -1 if the root set is full.
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

// Records a reference from `from` to `to`. Returns 0, or -1 if full.
int obj_add_ref(Obj *from, Obj *to) {
    if (from->ref_count == MAX_REFS) {
        return -1;
    }
    from->refs[from->ref_count++] = to;
    return 0;
}

static void mark(Obj *o) {
    // BUG: no visited check — in a cycle this recursion never ends
    // (the marked flag is sitting right there, unused as a guard).
    o->marked = true;
    // BUG: only the FIRST reference is followed; reachability is
    // transitive over ALL of them.
    if (o->ref_count > 0) {
        mark(o->refs[0]);
    }
}

// Mark from the roots, then sweep everything unmarked. (The sweep is
// correct: it unlinks as it frees, using a pointer-to-pointer walk.)
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
            free(o);
            h->live--;
        } else {
            link = &o->next;
        }
    }
}

// Frees every object, reachable or not.
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

    // A root holding TWO children, and a pair of objects that
    // reference each other (a cycle) — both everyday graph shapes.
    Obj *root = heap_new(&h, 1);
    Obj *left = heap_new(&h, 2);
    Obj *right = heap_new(&h, 3);
    heap_add_root(&h, root);
    obj_add_ref(root, left);
    obj_add_ref(root, right);

    Obj *a = heap_new(&h, 4);
    Obj *b = heap_new(&h, 5);
    heap_add_root(&h, a);
    obj_add_ref(a, b);
    obj_add_ref(b, a); /* the cycle */

    gc_collect(&h); /* the broken mark never returns from the cycle */
    printf("live after collect: %zu (expected 5)\n", h.live);

    heap_destroy(&h);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_unreachable_is_collected) {
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    heap_add_root(&h, root);
    heap_new(&h, 2); /* loose: no root, no reference */
    ASSERT_EQ(h.live, 2u);
    gc_collect(&h);
    ASSERT_EQ(h.live, 1u);
    heap_destroy(&h);
}

TEST(test_all_references_are_followed) {
    // A root with TWO children: reachability covers every reference,
    // not just refs[0].
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    Obj *left = heap_new(&h, 2);
    Obj *right = heap_new(&h, 3);
    heap_add_root(&h, root);
    ASSERT_EQ(obj_add_ref(root, left), 0);
    ASSERT_EQ(obj_add_ref(root, right), 0);
    gc_collect(&h);
    ASSERT_EQ(h.live, 3u);
    heap_destroy(&h);
}

TEST(test_reachability_is_transitive) {
    Heap h;
    heap_init(&h);
    Obj *root = heap_new(&h, 1);
    Obj *mid = heap_new(&h, 2);
    Obj *leaf = heap_new(&h, 3);
    heap_add_root(&h, root);
    obj_add_ref(root, mid);
    obj_add_ref(mid, leaf);
    gc_collect(&h);
    ASSERT_EQ(h.live, 3u);
    heap_destroy(&h);
}

TEST(test_cycles_terminate) {
    // Two objects referencing each other, one rooted: mark must visit
    // each node ONCE — the marked flag is the visited set.
    Heap h;
    heap_init(&h);
    Obj *a = heap_new(&h, 1);
    Obj *b = heap_new(&h, 2);
    heap_add_root(&h, a);
    obj_add_ref(a, b);
    obj_add_ref(b, a);
    gc_collect(&h);
    ASSERT_EQ(h.live, 2u);
    /* an unrooted cycle is garbage, cycles or not */
    heap_remove_root(&h, a);
    gc_collect(&h);
    ASSERT_EQ(h.live, 0u);
    heap_destroy(&h);
}

TEST(test_allocation_failure_is_reported) {
    Heap h;
    heap_init(&h);
    cmetal_fail_next_alloc();
    ASSERT(heap_new(&h, 1) == NULL);
    ASSERT_EQ(h.live, 0u);
    heap_destroy(&h);
}

int main(void) {
    RUN_TEST(test_unreachable_is_collected);
    RUN_TEST(test_all_references_are_followed);
    RUN_TEST(test_reachability_is_transitive);
    RUN_TEST(test_cycles_terminate);
    RUN_TEST(test_allocation_failure_is_reported);
    TEST_REPORT();
}
#endif
