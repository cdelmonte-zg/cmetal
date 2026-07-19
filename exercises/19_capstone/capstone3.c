// capstone3.c - Loading a binary format: own what you keep
//
// capstone2's view borrows from the file buffer — fine while the
// buffer lives, useless the moment it is freed or reused. A LOADER
// crosses that lifetime boundary: it validates (given correct, the
// previous exercise's discipline) and then copies everything it keeps
// into memory the chunk OWNS, decoding the constants into doubles on
// the way. From that point the file buffer is dead weight: free it,
// reuse it, the chunk must not care.
//
// This loader half-commits, one mindset with two symptoms:
//   1. the constants are dutifully copied — but the code is
//      "borrowed" from the buffer ("it's just bytes, they're right
//      there"). Overwrite or free the buffer and the chunk's code
//      changes under it (the demo frees the buffer and lets ASan
//      catch the read);
//   2. chunk_free, consistent with that mindset, frees only the
//      constants. Once the code IS an owned copy, that is a leak of
//      every loaded chunk's code (the leak sanitizer counts it at
//      demo exit).
//
// Contract: after chunk_load returns LOAD_OK, the chunk is
// independent of the buffer — its constants and code live in
// allocations the chunk owns, released by chunk_free. Allocation
// failures return ERR_ALLOC with nothing leaked and the chunk
// unchanged (allocations go through CMETAL_MALLOC).
//
// (Editorial note: "parse, then keep pointers into the input" is how
// configuration loaders and message routers acquire lifetime bugs.
// Copy at the border or document the borrow — this loader claims to
// copy, so it must.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "cmetal_alloc.h"

typedef struct {
    double *consts;
    size_t const_count;
    uint8_t *code;
    size_t code_len;
} Chunk;

#define LOAD_OK          0
#define ERR_BAD_MAGIC   -1
#define ERR_BAD_VERSION -2
#define ERR_TRUNCATED   -3
#define ERR_ALLOC       -4

#define CHUNK_MAGIC "CMBC"
#define CHUNK_VERSION 1

static inline uint16_t read_u16_le(const uint8_t *p) {
    return (uint16_t)(p[0] | (p[1] << 8));
}

static inline uint32_t read_u32_le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static inline uint64_t read_u64_le(const uint8_t *p) {
    uint64_t v = 0;
    for (int i = 0; i < 8; i++) {
        v |= (uint64_t)p[i] << (8 * i);
    }
    return v;
}

static inline double bits_to_double(uint64_t b) {
    double d;
    memcpy(&d, &b, sizeof d);
    return d;
}

// Validates and loads buf into an owning chunk. Returns LOAD_OK or
// the specific error; on error the chunk is unchanged and nothing is
// leaked.
int chunk_load(const uint8_t *buf, size_t len, Chunk *out) {
    /* validation: capstone2's border discipline, given correct */
    if (len < 8) {
        return ERR_TRUNCATED;
    }
    if (memcmp(buf, CHUNK_MAGIC, 4) != 0) {
        return ERR_BAD_MAGIC;
    }
    if (read_u16_le(buf + 4) != CHUNK_VERSION) {
        return ERR_BAD_VERSION;
    }
    uint16_t const_count = read_u16_le(buf + 6);
    size_t off = 8;
    if ((size_t)const_count > (len - off) / 8) {
        return ERR_TRUNCATED;
    }
    size_t consts_off = off;
    off += (size_t)const_count * 8;
    if (len - off < 4) {
        return ERR_TRUNCATED;
    }
    uint32_t code_len = read_u32_le(buf + off);
    off += 4;
    if ((size_t)code_len > len - off) {
        return ERR_TRUNCATED;
    }

    /* the lifetime crossing: copy what the chunk keeps */
    double *consts = CMETAL_MALLOC((size_t)const_count * sizeof(double));
    if (!consts) {
        return ERR_ALLOC;
    }
    for (size_t i = 0; i < const_count; i++) {
        consts[i] = bits_to_double(read_u64_le(buf + consts_off + i * 8));
    }

    // BUG: the code is "borrowed" straight from the buffer — the
    // loaded chunk claims to be independent of it, and is not. The
    // code must be an owned copy too (and if THAT allocation fails,
    // the constants above must be released before returning).
    out->consts = consts;
    out->const_count = const_count;
    out->code = (uint8_t *)(buf + off);
    out->code_len = code_len;
    return LOAD_OK;
}

void chunk_free(Chunk *c) {
    free(c->consts);
    // BUG: consistent with the borrow above, the code is not freed.
    // Once the code is an owned copy, this leaks it for every chunk
    // ever loaded.
    c->consts = NULL;
    c->const_count = 0;
    c->code = NULL;
    c->code_len = 0;
}

#ifndef TEST
int main(void) {
    static const uint8_t file[] = {
        'C', 'M', 'B', 'C', 0x01, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F,
        0x03, 0x00, 0x00, 0x00,
        1, 0, 0
    };
    uint8_t *buf = malloc(sizeof(file));
    if (!buf) {
        return 1;
    }
    memcpy(buf, file, sizeof(file));

    Chunk c = { NULL, 0, NULL, 0 };
    if (chunk_load(buf, sizeof(file), &c) != LOAD_OK) {
        free(buf);
        return 1;
    }

    // The chunk claims independence: the file buffer goes away.
    free(buf);

    // With borrowed code this reads freed memory — ASan points here.
    printf("code[0] after the buffer is gone: %d (expected 1)\n", c.code[0]);

    // And chunk_free must release everything the load allocated — the
    // leak sanitizer audits the exit.
    chunk_free(&c);
    return 0;
}
#else
#include "cmetal_test.h"

static const uint8_t GOOD[] = {
    'C', 'M', 'B', 'C', 0x01, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F,
    0x03, 0x00, 0x00, 0x00,
    1, 0, 0
};

TEST(test_load_decodes_the_chunk) {
    Chunk c = { NULL, 0, NULL, 0 };
    ASSERT_EQ(chunk_load(GOOD, sizeof(GOOD), &c), LOAD_OK);
    ASSERT_EQ(c.const_count, 1u);
    ASSERT(c.consts[0] == 1.5);
    ASSERT_EQ(c.code_len, 3u);
    ASSERT_EQ(c.code[0], 1);
    chunk_free(&c);
}

TEST(test_loaded_chunk_is_independent_of_the_buffer) {
    // Load, then trash the buffer: an owning chunk must not notice.
    uint8_t buf[sizeof(GOOD)];
    memcpy(buf, GOOD, sizeof(GOOD));
    Chunk c = { NULL, 0, NULL, 0 };
    ASSERT_EQ(chunk_load(buf, sizeof(buf), &c), LOAD_OK);
    memset(buf, 0xAA, sizeof(buf));
    ASSERT(c.consts[0] == 1.5);
    ASSERT_EQ(c.code[0], 1);
    ASSERT_EQ(c.code[1], 0);
    ASSERT_EQ(c.code[2], 0);
    chunk_free(&c);
}

TEST(test_validation_still_guards_the_border) {
    uint8_t file[sizeof(GOOD)];
    memcpy(file, GOOD, sizeof(GOOD));
    file[0] = 'X';
    Chunk c = { NULL, 0, NULL, 0 };
    ASSERT_EQ(chunk_load(file, sizeof(file), &c), ERR_BAD_MAGIC);
    memcpy(file, GOOD, sizeof(GOOD));
    file[16] = 0xFF; /* code_len larger than the buffer */
    ASSERT_EQ(chunk_load(file, sizeof(file), &c), ERR_TRUNCATED);
}

TEST(test_allocation_failure_is_clean) {
    Chunk c = { NULL, 0, NULL, 0 };
    cmetal_fail_next_alloc();
    ASSERT_EQ(chunk_load(GOOD, sizeof(GOOD), &c), ERR_ALLOC);
    ASSERT(c.consts == NULL);
    ASSERT(c.code == NULL);
    cmetal_alloc_reset();
}

int main(void) {
    RUN_TEST(test_load_decodes_the_chunk);
    RUN_TEST(test_loaded_chunk_is_independent_of_the_buffer);
    RUN_TEST(test_validation_still_guards_the_border);
    RUN_TEST(test_allocation_failure_is_clean);
    TEST_REPORT();
}
#endif
