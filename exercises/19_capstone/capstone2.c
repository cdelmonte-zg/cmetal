// capstone2.c - Reading a binary format: validate at the border
//
// The reader side of capstone1's format (magic "CMBC", version u16,
// const_count u16, constants as 8-byte IEEE-754 patterns, code_len
// u32, code — all little-endian). This parser produces a zero-copy
// VIEW: pointers into the caller's buffer, valid as long as the
// buffer is (owning copies are the next exercise).
//
// A file is input from outside the program: the border is where it
// must be checked, with the file's OWN claims treated as claims. This
// parser fails the border twice:
//   1. magic and version are never checked — any 8 bytes of anything
//      parse as a chunk, and a future-version file (whose layout may
//      differ) is happily misread;
//   2. the declared counts are trusted: a const_count larger than the
//      buffer sends the parser reading code_len from past the end,
//      and a code_len larger than the remainder yields a view that
//      points beyond the buffer (the demo walks it under ASan).
//
// Contract: chunk_parse returns PARSE_OK only for a well-formed
// buffer, ERR_BAD_MAGIC / ERR_BAD_VERSION / ERR_TRUNCATED otherwise
// (each malformation its own error — see bytecode1). Bytes after the
// code stream are ignored. The size arithmetic must not overflow:
// compare counts against what REMAINS, never add lengths together
// first (the arena1 lesson, at the file border).
//
// (Editorial note: image loaders, font parsers, database pages, save
// files — every "read the header, then trust it" bug you have read a
// CVE about is this exercise.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>

typedef struct {
    const uint8_t *consts; /* const_count 8-byte patterns */
    uint16_t const_count;
    const uint8_t *code;
    uint32_t code_len;
} ChunkView;

#define PARSE_OK         0
#define ERR_BAD_MAGIC   -1
#define ERR_BAD_VERSION -2
#define ERR_TRUNCATED   -3

#define CHUNK_MAGIC "CMBC"
#define CHUNK_VERSION 1

static inline uint16_t read_u16_le(const uint8_t *p) {
    return (uint16_t)(p[0] | (p[1] << 8));
}

static inline uint32_t read_u32_le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

// Parses buf into a zero-copy view. Returns PARSE_OK or the specific
// error.
int chunk_parse(const uint8_t *buf, size_t len, ChunkView *out) {
    if (len < 8) {
        return ERR_TRUNCATED;
    }
    // BUG: the magic and the version are never checked — any bytes
    // parse, and a version-2 file is misread as version 1.
    uint16_t const_count = read_u16_le(buf + 6);

    size_t off = 8;
    // BUG: const_count is the FILE's claim. Nothing checks that the
    // constants actually fit in the buffer before walking past them.
    out->consts = buf + off;
    out->const_count = const_count;
    off += (size_t)const_count * 8;

    // BUG: same for the code: code_len is read from wherever the
    // (unchecked) constants ended, and the resulting view may point
    // past the buffer.
    uint32_t code_len = read_u32_le(buf + off);
    off += 4;
    out->code = buf + off;
    out->code_len = code_len;
    return PARSE_OK;
}

#ifndef TEST
#include <stdlib.h>

int main(void) {
    // A file claiming 4 constants, in a heap buffer that holds one:
    // the parser must say TRUNCATED. The broken one builds a view
    // past the buffer — walking it makes ASan point at the read.
    static const uint8_t file[] = {
        'C', 'M', 'B', 'C', 0x01, 0x00, 0x04, 0x00, /* claims 4 */
        0, 0, 0, 0, 0, 0, 0, 0                      /* holds 1 */
    };
    uint8_t *buf = malloc(sizeof(file));
    if (!buf) {
        return 1;
    }
    memcpy(buf, file, sizeof(file));

    ChunkView v;
    int rc = chunk_parse(buf, sizeof(file), &v);
    printf("truncated constants: rc=%d (expected %d)\n", rc, ERR_TRUNCATED);
    if (rc == PARSE_OK) {
        unsigned sum = 0;
        for (size_t i = 0; i < (size_t)v.const_count * 8; i++) {
            sum += v.consts[i]; /* reads past the buffer */
        }
        printf("sum of claimed constants: %u\n", sum);
    }
    free(buf);
    return 0;
}
#else
#include "cmetal_test.h"

/* a well-formed file: 1 constant (1.5), 3 code bytes */
static const uint8_t GOOD[] = {
    'C', 'M', 'B', 'C', 0x01, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F,
    0x03, 0x00, 0x00, 0x00,
    1, 0, 0
};

TEST(test_well_formed_file_parses) {
    ChunkView v;
    ASSERT_EQ(chunk_parse(GOOD, sizeof(GOOD), &v), PARSE_OK);
    ASSERT_EQ(v.const_count, 1);
    ASSERT(v.consts == GOOD + 8);
    ASSERT_EQ(v.code_len, 3u);
    ASSERT(v.code == GOOD + 20);
}

TEST(test_bad_magic_is_reported) {
    uint8_t file[sizeof(GOOD)];
    memcpy(file, GOOD, sizeof(GOOD));
    file[0] = 'X';
    ChunkView v;
    ASSERT_EQ(chunk_parse(file, sizeof(file), &v), ERR_BAD_MAGIC);
}

TEST(test_unknown_version_is_reported) {
    // A version-2 file may have a different layout: reading it as
    // version 1 is misinterpretation, not compatibility.
    uint8_t file[sizeof(GOOD)];
    memcpy(file, GOOD, sizeof(GOOD));
    file[4] = 0x02;
    ChunkView v;
    ASSERT_EQ(chunk_parse(file, sizeof(file), &v), ERR_BAD_VERSION);
}

TEST(test_truncated_constants_are_reported) {
    /* claims 4 constants, holds 1 */
    uint8_t file[sizeof(GOOD)];
    memcpy(file, GOOD, sizeof(GOOD));
    file[6] = 0x04;
    ChunkView v;
    ASSERT_EQ(chunk_parse(file, sizeof(file), &v), ERR_TRUNCATED);
}

TEST(test_truncated_code_is_reported) {
    /* the code_len field claims more bytes than the buffer holds */
    uint8_t file[sizeof(GOOD)];
    memcpy(file, GOOD, sizeof(GOOD));
    file[16] = 0xFF; /* code_len 255 */
    ChunkView v;
    ASSERT_EQ(chunk_parse(file, sizeof(file), &v), ERR_TRUNCATED);
}

TEST(test_header_shorter_than_fixed_part_is_reported) {
    ChunkView v;
    ASSERT_EQ(chunk_parse(GOOD, 7, &v), ERR_TRUNCATED);
}

int main(void) {
    RUN_TEST(test_well_formed_file_parses);
    RUN_TEST(test_bad_magic_is_reported);
    RUN_TEST(test_unknown_version_is_reported);
    RUN_TEST(test_truncated_constants_are_reported);
    RUN_TEST(test_truncated_code_is_reported);
    RUN_TEST(test_header_shorter_than_fixed_part_is_reported);
    TEST_REPORT();
}
#endif
