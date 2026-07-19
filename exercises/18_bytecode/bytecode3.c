// bytecode3.c - Computed goto: the table is a contract nobody checks
//
// The portable dispatch loop is a switch (bytecode1/2). The classic
// alternative is COMPUTED GOTO, a GNU C extension both gcc and clang
// accept (this exercise builds with -std=gnu11): take the address of
// a label with &&label, store the addresses in a table indexed by
// opcode, and jump with goto *table[op]. One indirect jump per
// instruction, no bounds re-check by the compiler — and that is the
// point of the exercise, twice over:
//
//   1. the table is POSITIONAL. It must follow the enum order, and
//      nobody but you will notice it does not: here OP_MUL's slot
//      holds the negate handler and OP_NEG's slot holds multiply.
//      A switch would have paired case labels with code; the table
//      pairs them by POSITION only.
//   2. handlers do not "break". Control leaves a handler only via an
//      explicit DISPATCH() — the add handler below is missing its
//      dispatch, so execution falls straight through into the next
//      handler's code, exactly like a forgotten break in a switch,
//      but with no -Wimplicit-fallthrough to save you.
//
// The fetch/validation discipline (bounds, indices, unknown opcodes,
// stack checks) is given correct.
//
// Contract: same as bytecode1 — RUN_OK plus the result for
// well-formed programs, the specific error otherwise.
//
// (Editorial note: any function-pointer or label table indexed by an
// enum — message handlers, state machines, driver ops — carries the
// same two obligations: order in sync with the index space, and
// explicit control flow out of every entry.)

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>

enum {
    OP_HALT = 0,  /* stop; the top of the stack is the result */
    OP_CONST = 1, /* push consts[next byte] */
    OP_ADD = 2,   /* pop b, pop a, push a + b */
    OP_MUL = 3,   /* pop b, pop a, push a * b */
    OP_NEG = 4,   /* pop a, push -a */
    OP_COUNT
};

typedef struct {
    const uint8_t *code;
    size_t code_len;
    const double *consts;
    size_t const_count;
} Chunk;

#define RUN_OK          0
#define ERR_TRUNCATED  -1
#define ERR_BAD_CONST  -2
#define ERR_BAD_OPCODE -3
#define ERR_NO_HALT    -4
#define ERR_STACK      -5

#define STACK_MAX 16

int run(const Chunk *chunk, double *result) {
    // The dispatch table: one label address per opcode, indexed BY
    // the opcode value.
    // BUG: the last two entries are in the wrong order — OP_MUL (3)
    // jumps to the negate handler and OP_NEG (4) to multiply. The
    // compiler cannot know; only the enum order does.
    static const void *dispatch_table[OP_COUNT] = {
        &&do_halt, &&do_const, &&do_add, &&do_neg, &&do_mul
    };

    double stack[STACK_MAX];
    size_t sp = 0;
    size_t pc = 0;

    // Fetch, validate, jump. Control returns here only through
    // DISPATCH() — there is no loop around the handlers.
#define DISPATCH()                                                      \
    do {                                                                \
        if (pc >= chunk->code_len) {                                    \
            return ERR_NO_HALT;                                         \
        }                                                               \
        uint8_t op = chunk->code[pc++];                                 \
        if (op >= OP_COUNT) {                                          \
            return ERR_BAD_OPCODE;                                      \
        }                                                               \
        goto *dispatch_table[op];                                       \
    } while (0)

    DISPATCH();

do_halt:
    if (sp == 0) {
        return ERR_STACK;
    }
    *result = stack[sp - 1];
    return RUN_OK;

do_const: {
    if (pc >= chunk->code_len) {
        return ERR_TRUNCATED;
    }
    uint8_t idx = chunk->code[pc++];
    if (idx >= chunk->const_count) {
        return ERR_BAD_CONST;
    }
    if (sp == STACK_MAX) {
        return ERR_STACK;
    }
    stack[sp++] = chunk->consts[idx];
    DISPATCH();
}

do_add: {
    if (sp < 2) {
        return ERR_STACK;
    }
    double b = stack[--sp];
    double a = stack[--sp];
    stack[sp++] = a + b;
    // BUG: no DISPATCH() — control falls through into the multiply
    // handler, which pops two more operands that were never pushed.
}

do_mul: {
    if (sp < 2) {
        return ERR_STACK;
    }
    double b = stack[--sp];
    double a = stack[--sp];
    stack[sp++] = a * b;
    DISPATCH();
}

do_neg:
    if (sp < 1) {
        return ERR_STACK;
    }
    stack[sp - 1] = -stack[sp - 1];
    DISPATCH();

#undef DISPATCH
}

#ifndef TEST
int main(void) {
    /* (2 + 3) * 4 — one add, one multiply */
    static const uint8_t code[] = {
        OP_CONST, 0, OP_CONST, 1, OP_ADD, OP_CONST, 2, OP_MUL, OP_HALT
    };
    static const double consts[] = { 2.0, 3.0, 4.0 };
    Chunk chunk = { code, sizeof(code), consts, 3 };
    double result = 0.0;
    int rc = run(&chunk, &result);
    printf("(2 + 3) * 4: rc=%d result=%g (expected rc=0 result=20)\n",
           rc, result);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_add_then_halt) {
    /* 2 + 3: exercises the add handler's exit path in isolation */
    static const uint8_t code[] = { OP_CONST, 0, OP_CONST, 1, OP_ADD, OP_HALT };
    static const double consts[] = { 2.0, 3.0 };
    Chunk chunk = { code, sizeof(code), consts, 2 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == 5.0);
}

TEST(test_mul_dispatches_to_mul) {
    /* 2 * 4: if the table is out of order this negates instead */
    static const uint8_t code[] = { OP_CONST, 0, OP_CONST, 1, OP_MUL, OP_HALT };
    static const double consts[] = { 2.0, 4.0 };
    Chunk chunk = { code, sizeof(code), consts, 2 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == 8.0);
}

TEST(test_neg_dispatches_to_neg) {
    static const uint8_t code[] = { OP_CONST, 0, OP_NEG, OP_HALT };
    static const double consts[] = { 5.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == -5.0);
}

TEST(test_add_mul_combined) {
    /* (2 + 3) * 4 */
    static const uint8_t code[] = {
        OP_CONST, 0, OP_CONST, 1, OP_ADD, OP_CONST, 2, OP_MUL, OP_HALT
    };
    static const double consts[] = { 2.0, 3.0, 4.0 };
    Chunk chunk = { code, sizeof(code), consts, 3 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == 20.0);
}

TEST(test_stream_discipline_still_holds) {
    static const uint8_t truncated[] = { OP_CONST };
    static const uint8_t unknown[] = { 0x7f, OP_HALT };
    static const double consts[] = { 2.0 };
    double result = 0.0;
    Chunk t = { truncated, sizeof(truncated), consts, 1 };
    ASSERT_EQ(run(&t, &result), ERR_TRUNCATED);
    Chunk u = { unknown, sizeof(unknown), consts, 1 };
    ASSERT_EQ(run(&u, &result), ERR_BAD_OPCODE);
}

int main(void) {
    RUN_TEST(test_add_then_halt);
    RUN_TEST(test_mul_dispatches_to_mul);
    RUN_TEST(test_neg_dispatches_to_neg);
    RUN_TEST(test_add_mul_combined);
    RUN_TEST(test_stream_discipline_still_holds);
    TEST_REPORT();
}
#endif
