// nanbox1.c - NaN boxing: carve your space out of NaN, not all of it
//
// An IEEE-754 double has 11 exponent bits and 52 mantissa bits. When
// every exponent bit is 1 and the mantissa is nonzero, the value is
// Not-a-Number — and the mantissa bits are free payload. That is a
// 64-bit word that can carry EITHER a full double OR a small tagged
// value: the layout trick behind compact value slots.
//
// The catch this exercise is about: you do not own every NaN. The
// hardware produces one too — 0.0/0.0 yields the CANONICAL quiet NaN,
// exponent all ones plus the top mantissa bit (and on some CPUs the
// sign bit). This encoding claims exactly that pattern as its boxed
// space, so an ordinary arithmetic result gets misclassified as a
// boxed value. The boxed space must be a NaN subspace DISTINCT from
// the canonical NaNs those operations produce: set one more mantissa
// bit and the collision is gone.
//
// One honest limit, shared by every NaN-boxing scheme: arithmetic can
// PROPAGATE the payload of a NaN operand, so a crafted NaN carrying
// bit 50 can flow through an operation and land in the boxed
// subspace. Arbitrary NaN payloads are therefore OUTSIDE the
// representable double domain — the domain is all non-NaN doubles
// plus the canonical NaNs. Every encoding trades away something;
// the skill is saying precisely what.
//
// Second bug, smaller but just as real: the boolean tags are built
// with an off-by-one OR, so false lands on an unassigned pattern and
// true lands on nil.
//
// Contract: every double in the representable domain — all non-NaN
// doubles, plus the canonical NaNs invalid operations produce
// (0.0/0.0, either sign bit) — round-trips through
// box_double/as_double and satisfies is_double. nil, false and true
// are three distinct non-double values.
//
// (Editorial note: packing type + payload into one word is how value
// arrays, message slots and handle tables stay compact. Language
// runtimes box values this way, but nothing here requires one — this
// is IEEE-754 literacy plus mask discipline.)

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

_Static_assert(sizeof(double) == 8, "this exercise assumes 64-bit doubles");

typedef uint64_t Value;

// The only legal pun in town: memcpy between objects of the same
// size. Casting a double* to uint64_t* and dereferencing violates
// strict aliasing.
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

// The hardware's quiet NaN: exponent all ones + top mantissa bit.
#define QNAN ((uint64_t)0x7ff8000000000000)

// BUG: BOX_MASK claims EVERY quiet NaN as boxed space — but 0.0/0.0
// produces exactly the pattern QNAN. The boxed space must set one
// MORE mantissa bit (bit 50), a subspace the canonical NaNs stay out
// of.
#define BOX_MASK QNAN

#define NIL_VAL   (BOX_MASK | 1)
#define FALSE_VAL (BOX_MASK | 2)
#define TRUE_VAL  (BOX_MASK | 3)

// A Value is a double unless it lives in the boxed space.
bool is_double(Value v) {
    return (v & BOX_MASK) != BOX_MASK;
}

Value box_double(double d) {
    return double_to_bits(d);
}

double as_double(Value v) {
    return bits_to_double(v);
}

Value nil_val(void) {
    return NIL_VAL;
}

bool is_nil(Value v) {
    return v == NIL_VAL;
}

Value box_bool(bool b) {
    // BUG: off by one in the tag — false becomes BOX_MASK | 0 (an
    // unassigned pattern) and true becomes BOX_MASK | 1, which is
    // nil.
    return BOX_MASK | (uint64_t)b;
}

bool is_bool(Value v) {
    return v == FALSE_VAL || v == TRUE_VAL;
}

bool as_bool(Value v) {
    return v == TRUE_VAL;
}

#ifndef TEST
static const char *classify(Value v) {
    if (is_double(v)) {
        return "double";
    }
    if (is_nil(v)) {
        return "nil";
    }
    if (is_bool(v)) {
        return as_bool(v) ? "true" : "false";
    }
    return "unassigned pattern (!)";
}

int main(void) {
    printf("3.25        -> %s\n", classify(box_double(3.25)));
    printf("nil         -> %s\n", classify(nil_val()));
    printf("true        -> %s\n", classify(box_bool(true)));
    printf("false       -> %s\n", classify(box_bool(false)));

    // The hardware makes NaNs too. This one must STILL be a double.
    volatile double zero = 0.0;
    double n = zero / zero;
    printf("0.0/0.0     -> %s (must be double)\n", classify(box_double(n)));
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_doubles_round_trip) {
    double samples[] = { 0.0, -0.0, 1.0, -3.25, 1e300, 5e-324 };
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); i++) {
        Value v = box_double(samples[i]);
        ASSERT(is_double(v));
        ASSERT(as_double(v) == samples[i]);
    }
}

TEST(test_canonical_nan_is_still_a_double) {
    // 0.0/0.0 at runtime: the canonical quiet NaN (sign bit set on
    // some CPUs). This — not every conceivable NaN payload — is what
    // the representable domain includes.
    volatile double zero = 0.0;
    double n = zero / zero;
    Value v = box_double(n);
    ASSERT(is_double(v));
    double back = as_double(v);
    ASSERT(back != back); /* still NaN after the round trip */
    /* the canonical patterns, both signs, spelled out in bits */
    ASSERT(is_double(box_double(bits_to_double(QNAN))));
    ASSERT(is_double(box_double(bits_to_double((uint64_t)1 << 63 | QNAN))));
}

TEST(test_singletons_are_distinct_non_doubles) {
    Value nil = nil_val();
    Value f = box_bool(false);
    Value t = box_bool(true);
    ASSERT(!is_double(nil));
    ASSERT(!is_double(f));
    ASSERT(!is_double(t));
    ASSERT(is_nil(nil));
    ASSERT(is_bool(f));
    ASSERT(is_bool(t));
    ASSERT(!as_bool(f));
    ASSERT(as_bool(t));
    /* three values, three patterns */
    ASSERT(nil != f);
    ASSERT(nil != t);
    ASSERT(f != t);
    /* true is true, not nil */
    ASSERT(!is_nil(t));
}

int main(void) {
    RUN_TEST(test_doubles_round_trip);
    RUN_TEST(test_canonical_nan_is_still_a_double);
    RUN_TEST(test_singletons_are_distinct_non_doubles);
    TEST_REPORT();
}
#endif
