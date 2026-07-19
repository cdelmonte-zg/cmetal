/*
 * clings_alloc.h — allocation seam for clings exercises
 *
 * Exercises whose CONTRACT includes allocation failure ("returns -1 and
 * leaves the object untouched when growing fails") route their
 * allocations through CLINGS_MALLOC / CLINGS_REALLOC instead of calling
 * malloc / realloc directly.
 *
 * In normal builds the macros ARE malloc/realloc — zero overhead, no
 * behavior change. In TEST builds (-DTEST) they call a tiny injectable
 * allocator that a test can arm to fail on demand:
 *
 *     clings_fail_next_alloc();              // next allocation -> NULL
 *     ASSERT_EQ(dynarray_push(da, 3), -1);   // failure branch, for real
 *
 * This is the standard trick for making error paths testable: the code
 * under test doesn't know it's being lied to, and the failure is
 * deterministic instead of requiring actual memory exhaustion.
 */
#ifndef CLINGS_ALLOC_H
#define CLINGS_ALLOC_H

#include <stdlib.h>

#ifdef TEST

/* < 0: never fail; 0: fail the next allocation; > 0: countdown. */
static int clings_allocs_until_fail = -1;

/* Arm the allocator: the NEXT allocation returns NULL, then behavior
 * returns to normal automatically. */
static inline void clings_fail_next_alloc(void) {
    clings_allocs_until_fail = 0;
}

/* Disarm a pending failure (rarely needed: firing auto-disarms). */
static inline void clings_alloc_reset(void) {
    clings_allocs_until_fail = -1;
}

static inline int clings_alloc_should_fail(void) {
    if (clings_allocs_until_fail == 0) {
        clings_allocs_until_fail = -1;
        return 1;
    }
    if (clings_allocs_until_fail > 0) {
        clings_allocs_until_fail--;
    }
    return 0;
}

static inline void *clings_test_malloc(size_t size) {
    return clings_alloc_should_fail() ? NULL : malloc(size);
}

static inline void *clings_test_realloc(void *ptr, size_t size) {
    return clings_alloc_should_fail() ? NULL : realloc(ptr, size);
}

#define CLINGS_MALLOC  clings_test_malloc
#define CLINGS_REALLOC clings_test_realloc

#else /* !TEST */

#define CLINGS_MALLOC  malloc
#define CLINGS_REALLOC realloc

#endif

#endif /* CLINGS_ALLOC_H */
