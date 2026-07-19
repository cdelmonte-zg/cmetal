// tagged2.c - Exhaustive dispatch: make the compiler count your cases
//
// Dispatching on a tag is where tagged unions earn their keep — and
// where two classic bugs live:
//   1. a missing break lets one case FALL THROUGH into the next
//      (handle_message treats a PING as if it carried DATA). This one
//      the compiler already catches: -Wimplicit-fallthrough, part of
//      -Wextra, refuses to build this file as shipped;
//   2. a `default:` arm swallows every variant you forgot — today
//      that's MSG_CLOSE, tomorrow it's the variant you add next month,
//      silently "handled" by doing the wrong thing. No warning exists
//      for this one: only removing the default gives you one.
//
// The fix for bug 2 is not "add the missing case": it is REMOVING the
// default. With -Wall, gcc and clang warn (-Wswitch) whenever a switch
// on an enum misses a case and has no default — and -Werror turns that
// warning into a build failure. Delete the default and the compiler
// becomes your exhaustiveness checker, forever.
//
// (Editorial note: this is any event loop, protocol demux or state
// machine — nothing here requires an interpreter, though a bytecode
// dispatch loop is exactly this switch with more cases.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>

typedef enum {
    MSG_PING,
    MSG_DATA,
    MSG_CLOSE,
} MessageType;

typedef struct {
    MessageType type;
    union {
        struct {
            const uint8_t *bytes;
            size_t len;
        } data; /* MSG_DATA */
        struct {
            int code;
        } close; /* MSG_CLOSE */
    } as;
} Message;

// Contract:
//   PING  -> 1                  (answer with a pong)
//   DATA  -> (int)len           (bytes accepted)
//   CLOSE -> -code              (negative shutdown reason)
int handle_message(const Message *m) {
    int result = 0;
    switch (m->type) {
    case MSG_PING:
        result = 1;
        // BUG: missing break — a PING falls through and gets
        // reinterpreted as DATA (reading a union member that was
        // never written). -Wimplicit-fallthrough is already refusing
        // to compile this: the fix is one keyword away.
    case MSG_DATA:
        result = (int)m->as.data.len;
        break;
    default:
        // BUG: MSG_CLOSE lands here and is silently "handled" as 0 —
        // and so will every variant added in the future. Remove the
        // default and handle CLOSE explicitly: -Wswitch + -Werror
        // will then flag any future forgotten case at compile time.
        result = 0;
        break;
    }
    return result;
}

// Human-readable tag name, e.g. for logs.
const char *message_type_name(MessageType t) {
    switch (t) {
    case MSG_PING:
        return "ping";
    case MSG_DATA:
        return "data";
    default:
        // BUG: same disease — "close" never gets its name, and new
        // variants will silently log as "unknown" too.
        return "unknown";
    }
}

#ifndef TEST
int main(void) {
    const uint8_t payload[3] = {1, 2, 3};
    Message ping = {.type = MSG_PING};
    Message data = {.type = MSG_DATA, .as.data = {payload, sizeof(payload)}};
    Message close_msg = {.type = MSG_CLOSE, .as.close = {7}};

    printf("%s  -> %d\n", message_type_name(ping.type), handle_message(&ping));
    printf("%s  -> %d\n", message_type_name(data.type), handle_message(&data));
    printf("%s -> %d\n", message_type_name(close_msg.type), handle_message(&close_msg));
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_ping_is_ponged) {
    Message m = {.type = MSG_PING};
    ASSERT_EQ(handle_message(&m), 1);
}

TEST(test_data_reports_length) {
    const uint8_t payload[5] = {0};
    Message m = {.type = MSG_DATA, .as.data = {payload, 5}};
    ASSERT_EQ(handle_message(&m), 5);
}

TEST(test_close_reports_negative_code) {
    Message m = {.type = MSG_CLOSE, .as.close = {7}};
    ASSERT_EQ(handle_message(&m), -7);
}

TEST(test_every_type_has_a_name) {
    ASSERT_STR_EQ(message_type_name(MSG_PING), "ping");
    ASSERT_STR_EQ(message_type_name(MSG_DATA), "data");
    ASSERT_STR_EQ(message_type_name(MSG_CLOSE), "close");
}

int main(void) {
    RUN_TEST(test_ping_is_ponged);
    RUN_TEST(test_data_reports_length);
    RUN_TEST(test_close_reports_negative_code);
    RUN_TEST(test_every_type_has_a_name);
    TEST_REPORT();
}
#endif
