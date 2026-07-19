/*
 * cmetal_alloc.h — allocation seam for cmetal exercises
 *
 * Exercises whose CONTRACT includes allocation failure ("returns -1 and
 * leaves the object untouched when growing fails") route their
 * allocations through CMETAL_MALLOC / CMETAL_REALLOC instead of calling
 * malloc / realloc directly.
 *
 * In normal builds the macros ARE malloc/realloc — zero overhead, no
 * behavior change. In TEST builds (-DTEST) they call a tiny injectable
 * allocator that a test can arm to fail on demand:
 *
 *     cmetal_fail_next_alloc();              // next allocation -> NULL
 *     ASSERT_EQ(dynarray_push(da, 3), -1);   // failure branch, for real
 *
 * This is the standard trick for making error paths testable: the code
 * under test doesn't know it's being lied to, and the failure is
 * deterministic instead of requiring actual memory exhaustion.
 */
#ifndef CMETAL_ALLOC_H
#define CMETAL_ALLOC_H

#include <stdlib.h>

#ifdef TEST

/* < 0: never fail; 0: fail the next allocation; > 0: countdown. */
static int cmetal_allocs_until_fail = -1;

/* Arm the allocator: the NEXT allocation returns NULL, then behavior
 * returns to normal automatically. */
static inline void cmetal_fail_next_alloc(void) {
    cmetal_allocs_until_fail = 0;
}

/* Disarm a pending failure (rarely needed: firing auto-disarms). */
static inline void cmetal_alloc_reset(void) {
    cmetal_allocs_until_fail = -1;
}

static inline int cmetal_alloc_should_fail(void) {
    if (cmetal_allocs_until_fail == 0) {
        cmetal_allocs_until_fail = -1;
        return 1;
    }
    if (cmetal_allocs_until_fail > 0) {
        cmetal_allocs_until_fail--;
    }
    return 0;
}

static inline void *cmetal_test_malloc(size_t size) {
    return cmetal_alloc_should_fail() ? NULL : malloc(size);
}

static inline void *cmetal_test_realloc(void *ptr, size_t size) {
    return cmetal_alloc_should_fail() ? NULL : realloc(ptr, size);
}

#define CMETAL_MALLOC  cmetal_test_malloc
#define CMETAL_REALLOC cmetal_test_realloc

#else /* !TEST */

#define CMETAL_MALLOC  malloc
#define CMETAL_REALLOC realloc

#endif

#endif /* CMETAL_ALLOC_H */
