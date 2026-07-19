// bytecode1.c - The dispatch loop is a parser of untrusted input
//
// A bytecode interpreter is a loop: fetch an opcode, decode its
// operands, execute, repeat. Strip the vocabulary away and it is any
// processor of a command stream — a network protocol handler, a
// format decoder, an IPC endpoint. And the first rule of those is:
// THE STREAM IS DATA. It can end mid-instruction, index things that
// do not exist, or contain bytes you never defined. Each of those is
// an error to report, not an assumption to make.
//
// This loop trusts the stream three times:
//   1. OP_CONST fetches its operand byte without checking the stream
//      has one — a truncated program reads past the end of the code
//      buffer (ASan shows it on the demo);
//   2. the constant INDEX is used without validation — consts[200]
//      reads far outside the pool;
//   3. an unknown opcode is silently skipped, as if tolerant parsing
//      of an instruction stream were a kindness. It is corruption:
//      report and stop.
//
// Contract: run() returns RUN_OK with the result for a well-formed
// program, and the SPECIFIC error for each malformation — the caller
// can tell what went wrong. The stack helpers and the end-of-stream
// check are given correct.
//
// (Editorial note: every wire format and job queue needs exactly this
// discipline. Interpreters run on it too, but nothing here requires
// one.)

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
#define ERR_TRUNCATED  -1 /* stream ended mid-instruction */
#define ERR_BAD_CONST  -2 /* constant index outside the pool */
#define ERR_BAD_OPCODE -3 /* byte that is not an instruction */
#define ERR_NO_HALT    -4 /* stream ended without OP_HALT */
#define ERR_STACK      -5 /* stack over/underflow */

#define STACK_MAX 16

// Executes the chunk. On success returns RUN_OK and stores the value
// on top of the stack into *result.
int run(const Chunk *chunk, double *result) {
    double stack[STACK_MAX];
    size_t sp = 0;
    size_t pc = 0;

    while (pc < chunk->code_len) {
        uint8_t op = chunk->code[pc++];
        switch (op) {
        case OP_HALT:
            if (sp == 0) {
                return ERR_STACK;
            }
            *result = stack[sp - 1];
            return RUN_OK;
        case OP_CONST: {
            // BUG: no check that the stream still HAS an operand
            // byte — a truncated program reads past the code buffer.
            uint8_t idx = chunk->code[pc++];
            // BUG: idx comes from the stream; nothing says it is
            // inside the constant pool.
            if (sp == STACK_MAX) {
                return ERR_STACK;
            }
            stack[sp++] = chunk->consts[idx];
            break;
        }
        case OP_ADD: {
            if (sp < 2) {
                return ERR_STACK;
            }
            double b = stack[--sp];
            double a = stack[--sp];
            stack[sp++] = a + b;
            break;
        }
        case OP_MUL: {
            if (sp < 2) {
                return ERR_STACK;
            }
            double b = stack[--sp];
            double a = stack[--sp];
            stack[sp++] = a * b;
            break;
        }
        case OP_NEG:
            if (sp < 1) {
                return ERR_STACK;
            }
            stack[sp - 1] = -stack[sp - 1];
            break;
        default:
            // BUG: an unknown byte in an instruction stream is not
            // noise to skip — it is corruption to report.
            break;
        }
    }
    return ERR_NO_HALT;
}

#ifndef TEST
#include <stdlib.h>
#include <string.h>

int main(void) {
    // The demo feeds the loop a TRUNCATED program from a heap buffer
    // sized exactly to the stream: the missing-bounds fetch reads
    // past it, and AddressSanitizer names the byte.
    static const uint8_t prog[] = { OP_CONST };
    uint8_t *code = malloc(sizeof(prog));
    if (!code) {
        return 1;
    }
    memcpy(code, prog, sizeof(prog));

    static const double consts[] = { 2.0 };
    Chunk chunk = { code, sizeof(prog), consts, 1 };
    double result = 0.0;
    int rc = run(&chunk, &result);
    printf("truncated stream: rc=%d (expected %d)\n", rc, ERR_TRUNCATED);

    free(code);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_well_formed_program_runs) {
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

TEST(test_neg_works) {
    static const uint8_t code[] = { OP_CONST, 0, OP_NEG, OP_HALT };
    static const double consts[] = { 5.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == -5.0);
}

TEST(test_truncated_operand_is_reported) {
    // The stream ends where the operand should be. The error must say
    // TRUNCATED — not "no halt", which is what falls out of reading a
    // garbage byte and marching on.
    static const uint8_t code[] = { OP_CONST };
    static const double consts[] = { 2.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), ERR_TRUNCATED);
}

TEST(test_bad_constant_index_is_reported) {
    static const uint8_t code[] = { OP_CONST, 200, OP_HALT };
    static const double consts[] = { 2.0, 3.0 };
    Chunk chunk = { code, sizeof(code), consts, 2 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), ERR_BAD_CONST);
}

TEST(test_unknown_opcode_is_reported) {
    static const uint8_t code[] = { 0x7f, OP_HALT };
    static const double consts[] = { 2.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), ERR_BAD_OPCODE);
}

TEST(test_missing_halt_is_reported) {
    static const uint8_t code[] = { OP_CONST, 0 };
    static const double consts[] = { 2.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), ERR_NO_HALT);
}

int main(void) {
    RUN_TEST(test_well_formed_program_runs);
    RUN_TEST(test_neg_works);
    RUN_TEST(test_truncated_operand_is_reported);
    RUN_TEST(test_bad_constant_index_is_reported);
    RUN_TEST(test_unknown_opcode_is_reported);
    RUN_TEST(test_missing_halt_is_reported);
    TEST_REPORT();
}
#endif
