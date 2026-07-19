// tagged3.c - Ownership across variants: copies and transitions
//
// A tagged union whose variants OWN memory has two moments of truth:
//   - COPYING it: a struct assignment copies the pointer, not the
//     buffer — two owners for one allocation, and the second free is a
//     double free (the demo performs it; ASan names it);
//   - CHANGING its variant: overwriting a text slot with a number
//     without releasing the old buffer leaks it — the transition is
//     where the old payload's lifetime ends, so the transition must
//     free it.
//
// Contract:
//   - attr_copy produces an INDEPENDENT deep copy; it returns 0, or -1
//     if allocation fails, leaving *dst untouched. (Allocation goes
//     through CLINGS_MALLOC, so the tests can force that failure: a
//     shallow copy that allocates nothing cannot honor this contract.)
//   - attr_set_number / attr_set_text release the previous payload
//     before installing the new one.
//
// (Editorial note: this is any reusable slot that changes type over
// time — a spreadsheet cell, a UI property, a cache entry, a database
// row field. An interpreter's variables behave the same way, but
// nothing here requires one.)

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "clings_alloc.h"

typedef enum {
    ATTR_NONE,
    ATTR_NUMBER,
    ATTR_TEXT,
} AttrType;

typedef struct {
    AttrType type;
    union {
        double number;
        char *text; /* owned, heap-allocated */
    } as;
} Attr;

Attr attr_none(void) {
    return (Attr){.type = ATTR_NONE};
}

Attr attr_number(double n) {
    return (Attr){.type = ATTR_NUMBER, .as.number = n};
}

// Owns a copy of s; returns ATTR_NONE if allocation fails.
Attr attr_text(const char *s) {
    size_t len = strlen(s);
    char *copy = CLINGS_MALLOC(len + 1);
    if (!copy) {
        return attr_none();
    }
    memcpy(copy, s, len + 1);
    return (Attr){.type = ATTR_TEXT, .as.text = copy};
}

void attr_free(Attr *a) {
    if (a->type == ATTR_TEXT) {
        free(a->as.text);
    }
    *a = attr_none();
}

// Deep copy: *dst becomes an independent copy of *src.
// Returns 0 on success; -1 if allocation fails (*dst untouched).
int attr_copy(Attr *dst, const Attr *src) {
    // BUG: a struct assignment copies the text POINTER — src and dst
    // now both "own" the same buffer, and freeing both is a double
    // free. A deep copy must allocate.
    *dst = *src;
    return 0;
}

// Replaces whatever the slot holds with a number.
void attr_set_number(Attr *a, double n) {
    // BUG: if the slot held text, the old buffer is never released —
    // the variant transition is exactly where its lifetime ends.
    a->type = ATTR_NUMBER;
    a->as.number = n;
}

// Replaces whatever the slot holds with an owned copy of s
// (keeps the old value if allocation fails).
void attr_set_text(Attr *a, const char *s) {
    size_t len = strlen(s);
    char *copy = CLINGS_MALLOC(len + 1);
    if (!copy) {
        return;
    }
    memcpy(copy, s, len + 1);
    // BUG: same leak — the previous payload is dropped on the floor.
    a->type = ATTR_TEXT;
    a->as.text = copy;
}

#ifndef TEST
int main(void) {
    // A copy must be independent: freeing both the original and the
    // copy is the whole point of a DEEP copy — and a double free with
    // a shallow one.
    Attr label = attr_text("total");
    Attr copy = attr_none();
    if (attr_copy(&copy, &label) != 0) {
        return 1;
    }
    printf("copy: %s\n", copy.as.text);
    attr_free(&label);
    attr_free(&copy);

    // A transition ends the old payload's lifetime: this slot goes
    // text -> number, and the text buffer must be released — the
    // sanitizer's leak check watches the exit.
    Attr slot = attr_text("temporary");
    attr_set_number(&slot, 42.0);
    printf("slot: %g\n", slot.as.number);
    attr_free(&slot);

    return 0;
}
#else
#include "clings_test.h"

TEST(test_copy_is_deep) {
    Attr a = attr_text("shared?");
    Attr b = attr_none();
    ASSERT_EQ(attr_copy(&b, &a), 0);
    ASSERT_EQ(b.type, ATTR_TEXT);
    ASSERT(a.as.text != b.as.text); /* different buffers */
    ASSERT_STR_EQ(b.as.text, "shared?");
    attr_free(&a);
    ASSERT_STR_EQ(b.as.text, "shared?"); /* survives the original */
    attr_free(&b);
}

TEST(test_copy_failure_leaves_dst_untouched) {
    // A deep copy of text MUST allocate: force that allocation to fail
    // and the contract requires -1 with *dst untouched. A shallow copy
    // allocates nothing, "succeeds", and fails this test.
    Attr a = attr_text("needs a buffer");
    Attr b = attr_number(7.0);
    clings_fail_next_alloc();
    ASSERT_EQ(attr_copy(&b, &a), -1);
    ASSERT_EQ(b.type, ATTR_NUMBER);
    ASSERT(b.as.number == 7.0);
    attr_free(&a);
}

TEST(test_copy_of_number_needs_no_allocation) {
    Attr a = attr_number(3.5);
    Attr b = attr_none();
    clings_fail_next_alloc();
    ASSERT_EQ(attr_copy(&b, &a), 0); /* nothing to allocate */
    clings_alloc_reset();
    ASSERT_EQ(b.type, ATTR_NUMBER);
    ASSERT(b.as.number == 3.5);
}

TEST(test_transitions_replace_the_value) {
    Attr slot = attr_text("old");
    attr_set_number(&slot, 1.5);
    ASSERT_EQ(slot.type, ATTR_NUMBER);
    ASSERT(slot.as.number == 1.5);
    attr_set_text(&slot, "new");
    ASSERT_EQ(slot.type, ATTR_TEXT);
    ASSERT_STR_EQ(slot.as.text, "new");
    attr_free(&slot);
}

TEST(test_set_text_keeps_old_value_on_failure) {
    Attr slot = attr_text("keep me");
    clings_fail_next_alloc();
    attr_set_text(&slot, "never arrives");
    ASSERT_EQ(slot.type, ATTR_TEXT);
    ASSERT_STR_EQ(slot.as.text, "keep me");
    attr_free(&slot);
}

int main(void) {
    RUN_TEST(test_copy_is_deep);
    RUN_TEST(test_copy_failure_leaves_dst_untouched);
    RUN_TEST(test_copy_of_number_needs_no_allocation);
    RUN_TEST(test_transitions_replace_the_value);
    RUN_TEST(test_set_text_keeps_old_value_on_failure);
    TEST_REPORT();
}
#endif
