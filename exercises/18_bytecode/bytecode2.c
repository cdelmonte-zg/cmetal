// bytecode2.c - The stack is bounded, and it has two failure ends
//
// The evaluation stack of this little machine is an ordinary local
// array — like the explicit stacks in an undo system, an iterative
// tree walk, or an RPN calculator. A bounded stack fails two ways:
// POP from empty (the program asked for operands that are not there)
// and PUSH past capacity (the program nested deeper than the machine
// allows). Both are properties of the INPUT, so both must be errors
// the caller can see — not memory operations beyond the array.
//
// The helpers below do neither check. pop on an empty stack reads
// below the array and drives sp to (size_t)-1; the 17th push writes
// past the end. The fetch/decode loop is given correct (bounds,
// indices, unknown opcodes — see bytecode1); the stack discipline is
// the whole exercise. Fixing it will change the helpers' shape:
// push/pop must be able to REFUSE.
//
// Contract: a program that pops more than it pushed, or pushes more
// than STACK_MAX, makes run() return ERR_STACK. A program that uses
// exactly STACK_MAX slots is legal.
//
// (Editorial note: every explicit stack in production code — parser
// state, DFS frontiers, undo logs — needs refusable push/pop. The
// bytecode machine is just the smallest honest host for the lesson.)

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

// BUG: pushes with no room check — the 17th push writes past the end
// of the caller's array. A push must be able to refuse.
static void push(double *stack, size_t *sp, double v) {
    stack[(*sp)++] = v;
}

// BUG: pops with no emptiness check — on an empty stack this reads
// below the array and wraps sp to (size_t)-1. A pop must be able to
// refuse.
static double pop(double *stack, size_t *sp) {
    return stack[--*sp];
}

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
            if (pc >= chunk->code_len) {
                return ERR_TRUNCATED;
            }
            uint8_t idx = chunk->code[pc++];
            if (idx >= chunk->const_count) {
                return ERR_BAD_CONST;
            }
            push(stack, &sp, chunk->consts[idx]);
            break;
        }
        case OP_ADD: {
            double b = pop(stack, &sp);
            double a = pop(stack, &sp);
            push(stack, &sp, a + b);
            break;
        }
        case OP_MUL: {
            double b = pop(stack, &sp);
            double a = pop(stack, &sp);
            push(stack, &sp, a * b);
            break;
        }
        case OP_NEG:
            push(stack, &sp, -pop(stack, &sp));
            break;
        default:
            return ERR_BAD_OPCODE;
        }
    }
    return ERR_NO_HALT;
}

#ifndef TEST
int main(void) {
    // OP_ADD on an empty stack: the unchecked pop reads below the
    // stack array — AddressSanitizer points at it.
    static const uint8_t code[] = { OP_ADD, OP_HALT };
    static const double consts[] = { 1.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    int rc = run(&chunk, &result);
    printf("pop from empty stack: rc=%d (expected %d)\n", rc, ERR_STACK);
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

TEST(test_underflow_is_reported) {
    static const uint8_t add_on_empty[] = { OP_ADD, OP_HALT };
    static const uint8_t neg_on_empty[] = { OP_NEG, OP_HALT };
    static const double consts[] = { 1.0 };
    double result = 0.0;
    Chunk a = { add_on_empty, sizeof(add_on_empty), consts, 1 };
    ASSERT_EQ(run(&a, &result), ERR_STACK);
    Chunk n = { neg_on_empty, sizeof(neg_on_empty), consts, 1 };
    ASSERT_EQ(run(&n, &result), ERR_STACK);
}

TEST(test_full_stack_is_legal) {
    /* exactly STACK_MAX pushes, then halt: legal */
    uint8_t code[STACK_MAX * 2 + 1];
    for (int i = 0; i < STACK_MAX; i++) {
        code[i * 2] = OP_CONST;
        code[i * 2 + 1] = 0;
    }
    code[STACK_MAX * 2] = OP_HALT;
    static const double consts[] = { 7.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), RUN_OK);
    ASSERT(result == 7.0);
}

TEST(test_overflow_is_reported) {
    /* one push too many */
    uint8_t code[(STACK_MAX + 1) * 2 + 1];
    for (int i = 0; i < STACK_MAX + 1; i++) {
        code[i * 2] = OP_CONST;
        code[i * 2 + 1] = 0;
    }
    code[(STACK_MAX + 1) * 2] = OP_HALT;
    static const double consts[] = { 7.0 };
    Chunk chunk = { code, sizeof(code), consts, 1 };
    double result = 0.0;
    ASSERT_EQ(run(&chunk, &result), ERR_STACK);
}

int main(void) {
    RUN_TEST(test_well_formed_program_runs);
    RUN_TEST(test_underflow_is_reported);
    RUN_TEST(test_full_stack_is_legal);
    RUN_TEST(test_overflow_is_reported);
    TEST_REPORT();
}
#endif
