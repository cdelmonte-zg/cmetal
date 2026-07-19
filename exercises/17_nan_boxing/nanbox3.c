// nanbox3.c - Boxing pointers: 48 bits, the sign bit, and the mask cut
//
// On the mainstream configurations of today's 64-bit machines,
// user-space addresses fit in the low 48 bits — so those pointers fit
// in the NaN payload with room to spare. (Larger address spaces
// exist: x86-64 five-level paging reaches 57 bits, and high-half
// addresses would need sign extension, not zero-fill, to come back.
// This encoding covers pointers whose uintptr_t representation fits
// in the low 48 bits, and SAYS so — scoping the claim is part of the
// technique.) A Value is a pointer iff sign bit + boxed space are
// both set.
//
// Two bugs, both about respecting the payload:
//
//   1. box_ptr truncates the pointer to 32 bits — "it fits on my
//      machine" thinking. Heap addresses on the platforms you use
//      today do NOT fit in 32 bits; the boxed pointer comes back
//      pointing somewhere else entirely.
//   2. as_ptr returns the word WITHOUT stripping the tag bits — the
//      "pointer" it yields has 0xfffc in its top bits. The demo
//      dereferences it and dies far from the actual bug: a masking
//      mistake at the encode/decode boundary.
//
// Contract: box_ptr/as_ptr round-trip pointers whose uintptr_t
// representation fits in the low 48 bits (the address is preserved
// bit-for-bit), is_ptr holds exactly for boxed pointers, and doubles
// in the representable domain — ordinary values and the canonical
// NaNs, negative sign included — are never classified as pointers.
//
// (Editorial note: stuffing pointers into spare bits shows up in
// handle tables, lock-free algorithms and file formats. The lessons —
// widths are contracts, masks must be symmetric on both sides of an
// encode/decode pair — apply to every packed representation.)

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <stdlib.h>

_Static_assert(sizeof(double) == 8, "this exercise assumes 64-bit doubles");
_Static_assert(sizeof(void *) == 8, "this exercise assumes a 64-bit target");
_Static_assert(UINTPTR_MAX == UINT64_MAX,
               "this exercise assumes 64-bit uintptr_t");

typedef uint64_t Value;

static inline uint64_t double_to_bits(double d) {
    uint64_t b;
    memcpy(&b, &d, sizeof b);
    return b;
}

static inline double bits_to_double(uint64_t b) {
    double d;
    memcpy(&d, &b, sizeof d);
    return d;
}

// Boxed space as in nanbox1/2: quiet NaN + mantissa bit 50.
#define BOX_MASK ((uint64_t)0x7ffc000000000000)
#define SIGN_BIT ((uint64_t)1 << 63)

// A pointer wears sign bit + boxed space; the address is the payload.
#define PTR_TAG (SIGN_BIT | BOX_MASK)

bool is_double(Value v) {
    return (v & BOX_MASK) != BOX_MASK;
}

Value box_double(double d) {
    return double_to_bits(d);
}

double as_double(Value v) {
    return bits_to_double(v);
}

bool is_ptr(Value v) {
    return (v & PTR_TAG) == PTR_TAG;
}

Value box_ptr(void *p) {
    // BUG: truncates the address to its low 32 bits. The contract
    // covers 48 payload bits — the mask must cover all of them (the
    // tag occupies the top bits, so a low-48-bit address fits whole).
    return PTR_TAG | ((uint64_t)(uintptr_t)p & 0xffffffff);
}

void *as_ptr(Value v) {
    // BUG: the tag bits are still set — this "pointer" has 0xfffc in
    // its top bits. Decoding must strip exactly what encoding added.
    return (void *)(uintptr_t)v;
}

#ifndef TEST
int main(void) {
    int *n = malloc(sizeof(int));
    if (!n) {
        return 1;
    }
    *n = 41;

    Value v = box_ptr(n);
    printf("is_ptr: %d\n", is_ptr(v));

    // Decode and dereference: with broken masking this pointer is not
    // the one we boxed — the crash happens HERE, far from the bug.
    int *back = as_ptr(v);
    *back += 1;
    printf("boxed round trip: *back = %d (expected 42)\n", *back);

    free(n);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_pointer_round_trip) {
    int x = 7;
    Value v = box_ptr(&x);
    ASSERT(is_ptr(v));
    ASSERT(!is_double(v));
    ASSERT(as_ptr(v) == (void *)&x);
}

TEST(test_heap_pointer_survives_boxing) {
    int *n = malloc(sizeof(int));
    ASSERT(n != NULL);
    *n = 41;
    Value v = box_ptr(n);
    int *back = as_ptr(v);
    ASSERT(back == n);
    *back += 1;
    ASSERT_EQ(*n, 42);
    free(n);
}

TEST(test_full_48_bit_addresses_survive) {
    // A synthetic address using the full 48-bit contract range: every
    // payload bit must survive the round trip, not just the low 32.
    void *high = (void *)(uintptr_t)0x00007f12345678f0;
    Value v = box_ptr(high);
    ASSERT(is_ptr(v));
    ASSERT(as_ptr(v) == high);
}

TEST(test_representable_doubles_are_never_pointers) {
    ASSERT(!is_ptr(box_double(3.25)));
    ASSERT(!is_ptr(box_double(-3.25)));
    /* the canonical NaNs, both signs: sign bit + quiet NaN is NOT
     * enough to be a pointer — bit 50 makes the difference */
    ASSERT(!is_ptr(box_double(bits_to_double((uint64_t)0x7ff8000000000000))));
    ASSERT(!is_ptr(box_double(bits_to_double((uint64_t)0xfff8000000000000))));
    volatile double zero = 0.0;
    double n = zero / zero;
    ASSERT(is_double(box_double(n)));
    ASSERT(!is_ptr(box_double(n)));
}

int main(void) {
    RUN_TEST(test_pointer_round_trip);
    RUN_TEST(test_heap_pointer_survives_boxing);
    RUN_TEST(test_full_48_bit_addresses_survive);
    RUN_TEST(test_representable_doubles_are_never_pointers);
    TEST_REPORT();
}
#endif
