// encodings1.c - Endian-safe integer encoding
//
// Serialization means deciding what every BYTE looks like. The lazy way
// out — casting the buffer pointer and storing the whole integer —
// breaks three ways at once:
//   1. it writes the HOST's byte order, so the format silently changes
//      meaning across machines (these tests pin little-endian as the
//      FORMAT, byte by byte);
//   2. the cast violates strict aliasing (C11 6.5p7);
//   3. the store may be misaligned — undefined behavior that UBSan
//      reports on the spot (the demo writes at an odd offset).
//
// The portable way: move bytes with shifts. That is how every real
// format writer does it.
//
// TODO: rewrite both functions to move BYTES, not a whole uint32_t.
// Watch the read side: buf[3] is promoted to SIGNED int before the
// shift — for bytes >= 0x80, << 24 overflows int (see ub3). Cast to
// uint32_t before shifting.
//
// (Honesty note: on our little-endian targets a memcpy of the whole
// value would pass these tests too, but it would still be host-order
// dependent — wrong on a big-endian machine. The byte-content tests
// define the format; the shift solution is the only order-independent
// one. See CONTRIBUTING on language vs platform.)

#include <stdio.h>
#include <stdint.h>

// Writes value into buf[0..3], least significant byte FIRST, no matter
// what the host's byte order is. buf may be UNALIGNED (any offset
// inside a larger buffer).
void write_u32_le(uint8_t *buf, uint32_t value) {
    // BUG: host byte order + aliasing violation + unaligned store.
    *(uint32_t *)buf = value;
}

// Reads the value back from buf[0..3], little-endian, unaligned-safe.
uint32_t read_u32_le(const uint8_t *buf) {
    // BUG: buf[3] << 24 happens in signed int after promotion.
    return (uint32_t)(buf[0] | (buf[1] << 8) | (buf[2] << 16) | (buf[3] << 24));
}

#ifndef TEST
int main(void) {
    uint8_t frame[16] = {0};

    // Aligned round-trip.
    write_u32_le(frame, 0x12345678u);
    printf("aligned:    %08X\n", read_u32_le(frame));

    // Unaligned round-trip: real formats put integers wherever the
    // layout says, not where the CPU would like them.
    write_u32_le(frame + 1, 0xDEADBEEFu);
    printf("unaligned:  %08X\n", read_u32_le(frame + 1));

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_format_is_little_endian) {
    uint8_t buf[4] = {0};
    write_u32_le(buf, 0x12345678u);
    ASSERT_EQ(buf[0], 0x78);
    ASSERT_EQ(buf[1], 0x56);
    ASSERT_EQ(buf[2], 0x34);
    ASSERT_EQ(buf[3], 0x12);
}

TEST(test_read_known_bytes) {
    const uint8_t buf[4] = {0xEF, 0xBE, 0xAD, 0xDE};
    ASSERT_EQ(read_u32_le(buf), 0xDEADBEEFu);
}

TEST(test_roundtrip_boundaries) {
    uint8_t buf[4];
    write_u32_le(buf, 0u);
    ASSERT_EQ(read_u32_le(buf), 0u);
    write_u32_le(buf, UINT32_MAX);
    ASSERT_EQ(read_u32_le(buf), UINT32_MAX);
}

TEST(test_unaligned_offsets) {
    // Every offset inside a frame must work: the functions promise
    // unaligned-safe access.
    uint8_t frame[16];
    for (int off = 0; off < 5; off++) {
        write_u32_le(frame + off, 0xCAFEBABEu);
        ASSERT_EQ(read_u32_le(frame + off), 0xCAFEBABEu);
    }
}

int main(void) {
    RUN_TEST(test_format_is_little_endian);
    RUN_TEST(test_read_known_bytes);
    RUN_TEST(test_roundtrip_boundaries);
    RUN_TEST(test_unaligned_offsets);
    TEST_REPORT();
}
#endif
