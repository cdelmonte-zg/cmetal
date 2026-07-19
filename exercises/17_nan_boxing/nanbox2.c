// nanbox2.c - Payload discipline: sign extension and the equality trap
//
// This encoding packs a 32-bit integer into the NaN payload: the tag
// lives in bits 48-49, the integer in bits 0-31. Two lessons, one per
// bug:
//
//   1. Widening a NEGATIVE int does not just copy 32 bits — the cast
//      to uint64_t sign-extends, flooding bits 32-63 with ones. The
//      tag field is in that flood zone: box_int(-5) comes out wearing
//      every tag at once. Mask the payload to exactly the bits you
//      mean.
//   2. "One word, one compare" is tempting: value_equal as bit
//      equality. But doubles compare by VALUE, not by bits: 0.0 and
//      -0.0 are numerically equal with different bits, and NaN is
//      unequal to itself with identical bits. Equality needs a type
//      dispatch before it needs a comparison.
//
// Contract: box_int/as_int round-trip every int32_t, is_int holds for
// all of them, and int values are disjoint from doubles. value_equal
// compares doubles numerically and everything else by identity.
//
// (Editorial note: sign-extension floods and memcmp-style equality on
// float-bearing data corrupt hash keys, dedup passes and cache
// lookups in any codebase — no interpreter required.)

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

_Static_assert(sizeof(double) == 8, "this exercise assumes 64-bit doubles");

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

// The boxed space: quiet NaN plus mantissa bit 50, a pattern no
// arithmetic produces (see nanbox1).
#define BOX_MASK ((uint64_t)0x7ffc000000000000)

// Tag field: bits 48-49. Payload: bits 0-31.
#define TAG_MASK ((uint64_t)3 << 48)
#define TAG_INT  ((uint64_t)1 << 48)

bool is_double(Value v) {
    return (v & BOX_MASK) != BOX_MASK;
}

Value box_double(double d) {
    return double_to_bits(d);
}

double as_double(Value v) {
    return bits_to_double(v);
}

bool is_int(Value v) {
    return (v & (BOX_MASK | TAG_MASK)) == (BOX_MASK | TAG_INT);
}

Value box_int(int32_t i) {
    // BUG: the cast to uint64_t sign-extends. For negative i, bits
    // 32-63 all become 1 — the tag field reads 3, not TAG_INT, and
    // the sign bit is set too. The payload must be masked to its 32
    // bits.
    return BOX_MASK | TAG_INT | (uint64_t)i;
}

int32_t as_int(Value v) {
    return (int32_t)(uint32_t)(v & 0xffffffff);
}

bool value_equal(Value a, Value b) {
    // BUG: bit equality is not value equality. 0.0 == -0.0 with
    // different bits; NaN != NaN with identical bits. Doubles need a
    // numeric comparison — everything else compares by identity.
    return a == b;
}

#ifndef TEST
int main(void) {
    Value pos = box_int(42);
    Value neg = box_int(-5);
    printf("box_int(42):  is_int=%d  as_int=%d\n", is_int(pos), as_int(pos));
    printf("box_int(-5):  is_int=%d  as_int=%d (is_int must be 1)\n",
           is_int(neg), as_int(neg));

    printf("0.0 equals -0.0:   %d (must be 1)\n",
           value_equal(box_double(0.0), box_double(-0.0)));
    volatile double zero = 0.0;
    double n = zero / zero;
    printf("NaN equals itself: %d (must be 0)\n",
           value_equal(box_double(n), box_double(n)));
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_ints_round_trip) {
    int32_t samples[] = { 0, 1, 42, -1, -5, INT32_MAX, INT32_MIN };
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); i++) {
        Value v = box_int(samples[i]);
        ASSERT(is_int(v));
        ASSERT(!is_double(v));
        ASSERT_EQ(as_int(v), samples[i]);
    }
}

TEST(test_negative_ints_keep_their_tag) {
    // The sign-extension flood: -5 widened without a mask sets the
    // whole tag field.
    Value v = box_int(-5);
    ASSERT(is_int(v));
    ASSERT_EQ((int)((v & TAG_MASK) >> 48), (int)(TAG_INT >> 48));
}

TEST(test_doubles_compare_by_value) {
    ASSERT(value_equal(box_double(0.0), box_double(-0.0)));
    ASSERT(value_equal(box_double(1.5), box_double(1.5)));
    ASSERT(!value_equal(box_double(1.5), box_double(2.5)));
    /* NaN is unequal to itself — even bit-for-bit the same NaN */
    volatile double zero = 0.0;
    double n = zero / zero;
    ASSERT(!value_equal(box_double(n), box_double(n)));
}

TEST(test_non_doubles_compare_by_identity) {
    ASSERT(value_equal(box_int(7), box_int(7)));
    ASSERT(!value_equal(box_int(7), box_int(8)));
    /* an int is never equal to the double with the same numeric value:
     * different types, different bits */
    ASSERT(!value_equal(box_int(0), box_double(0.0)));
}

int main(void) {
    RUN_TEST(test_ints_round_trip);
    RUN_TEST(test_negative_ints_keep_their_tag);
    RUN_TEST(test_doubles_compare_by_value);
    RUN_TEST(test_non_doubles_compare_by_identity);
    TEST_REPORT();
}
#endif
