// hash1.c - FNV-1a: a hash function you can actually verify
//
// Before a hash TABLE there is a hash FUNCTION. FNV-1a is the little
// workhorse used everywhere from build tools to interpreters: for each
// byte, XOR it in, then multiply by the FNV prime. Two details carry
// all the correctness, and this file gets both wrong:
//   1. the operation ORDER: multiply-then-xor is a different algorithm
//      (FNV-1) with different output — every published FNV-1a test
//      vector disagrees with it;
//   2. the BYTE: plain `char` may be signed — and IS signed on the
//      current CI targets — so bytes >= 0x80 sign-extend before the
//      XOR. The same input then hashes differently on signed-char and
//      unsigned-char platforms: the bug is precisely the dependence
//      on an implementation-defined choice.
//
// (Editorial note: content fingerprints, dedup, cache keys, dispatch
// on strings — a hash function is everywhere. Interpreters use this
// exact one for identifiers, but nothing here requires one.)
//
// TODO: fix the order (xor first, then multiply) and hash UNSIGNED
// bytes. The tests pin published FNV-1a vectors, including one with a
// byte >= 0x80.

#include <stdio.h>
#include <stdint.h>

#define FNV_OFFSET 2166136261u
#define FNV_PRIME 16777619u

// FNV-1a over the bytes of a NUL-terminated string.
uint32_t fnv1a(const char *s) {
    uint32_t hash = FNV_OFFSET;
    for (const char *p = s; *p != '\0'; p++) {
        // BUG: FNV-1a is xor-THEN-multiply; this is the other one.
        hash *= FNV_PRIME;
        // BUG: *p is plain char (signed here) — bytes >= 0x80
        // sign-extend to 0xFFFFFFxx before the xor.
        hash ^= (uint32_t)*p;
    }
    return hash;
}

#ifndef TEST
int main(void) {
    const char *samples[] = {"", "a", "foobar", "hello", "caf\xE8"};
    for (int i = 0; i < 5; i++) {
        printf("fnv1a(\"%s\") = 0x%08X\n", samples[i], fnv1a(samples[i]));
    }
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_published_vectors) {
    // Straight from the FNV reference vectors.
    ASSERT_EQ(fnv1a(""), 0x811C9DC5u);
    ASSERT_EQ(fnv1a("a"), 0xE40C292Cu);
    ASSERT_EQ(fnv1a("foobar"), 0xBF9CF968u);
    ASSERT_EQ(fnv1a("hello"), 0x4F9F2CABu);
}

TEST(test_high_bit_bytes) {
    // "caf\xE8" — an arbitrary byte sequence containing 0xE8: the
    // hash operates on unsigned BYTES (no text encoding involved),
    // so 0xE8 must be hashed as 0xE8, not as 0xFFFFFFE8.
    ASSERT_EQ(fnv1a("caf\xE8"), 0x3408C00Fu);
}

TEST(test_different_strings_differ) {
    ASSERT_NE(fnv1a("port"), fnv1a("path"));
    ASSERT_NE(fnv1a("aa"), fnv1a("ab"));
}

int main(void) {
    RUN_TEST(test_published_vectors);
    RUN_TEST(test_high_bit_bytes);
    RUN_TEST(test_different_strings_differ);
    TEST_REPORT();
}
#endif
