// structs1.c - Struct layout and sizeof
//
// The compiler inserts padding bytes between struct fields to satisfy
// alignment requirements. The order of fields affects how much padding
// is added, and therefore the total size of the struct.
//
// Fix the padded struct by reordering its fields to minimize padding.

#include <stdio.h>
#include <stddef.h>

// NOTE on portability: sizes and alignments are ABI facts, not C rules.
// C11 only guarantees that members are laid out in declaration order
// and that there is no padding before the first one. The numbers in
// this exercise assume the mainstream 64-bit ABIs cmetal targets
// (x86-64 and AArch64 on Linux/macOS); these asserts make that
// assumption explicit.
_Static_assert(sizeof(int) == 4 && sizeof(double) == 8,
               "structs1 assumes 4-byte int and 8-byte double");
_Static_assert(_Alignof(double) == 8,
               "structs1 assumes 8-byte-aligned double");

// A tightly packed 2D point: two ints, no padding needed.
typedef struct {
    int x;
    int y;
} packed_point_t;

// BUG: These fields are ordered poorly, causing excessive padding.
// On the ABIs pinned above:
//   char (1 byte) + 7 padding + double (8 bytes) + int (4 bytes) + 4 padding = 24 bytes
//
// TODO: Reorder the fields so the struct uses minimal space: 16 bytes
// on these ABIs. Hint: place fields from largest alignment to smallest
// — an ABI heuristic that minimizes padding, not a rule of the C
// language itself.
typedef struct {
    char flag;
    double value;
    int count;
} padded_record_t;

// Returns the size of padded_record_t
size_t record_size(void) {
    return sizeof(padded_record_t);
}

// Fills in a padded_record_t and returns it
padded_record_t make_record(double value, int count, char flag) {
    padded_record_t r;
    r.value = value;
    r.count = count;
    r.flag = flag;
    return r;
}

#ifndef TEST
int main(void) {
    printf("sizeof(packed_point_t)  = %zu\n", sizeof(packed_point_t));
    printf("sizeof(padded_record_t) = %zu\n", sizeof(padded_record_t));

    padded_record_t r = make_record(3.14, 42, 'A');
    printf("record: value=%.2f count=%d flag=%c\n", r.value, r.count, r.flag);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_packed_point_size) {
    // Two ints should be exactly 8 bytes (no padding needed)
    ASSERT_EQ(sizeof(packed_point_t), 2 * sizeof(int));
}

TEST(test_record_optimized_size) {
    // After reordering: double(8) + int(4) + char(1) + 3 tail padding
    // to 8-byte alignment = 16 — justified by the _Static_asserts at
    // the top of this file.
    ASSERT(record_size() <= 16);
}

TEST(test_record_field_values) {
    padded_record_t r = make_record(2.718, 100, 'Z');
    ASSERT(r.value > 2.71 && r.value < 2.72);
    ASSERT_EQ(r.count, 100);
    ASSERT_EQ(r.flag, 'Z');
}

TEST(test_point_fields) {
    packed_point_t p;
    p.x = -5;
    p.y = 10;
    ASSERT_EQ(p.x, -5);
    ASSERT_EQ(p.y, 10);
}

int main(void) {
    RUN_TEST(test_packed_point_size);
    RUN_TEST(test_record_optimized_size);
    RUN_TEST(test_record_field_values);
    RUN_TEST(test_point_fields);
    TEST_REPORT();
}
#endif
