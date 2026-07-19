// tagged1.c - Tag discipline: the union is only as safe as its tag
//
// A tagged union pairs an enum with a union: the tag says which member
// is alive. Nothing else enforces it — every access that skips the tag
// check reads a member that was never written.
//
// (Editorial note: this is any config or JSON-ish scalar — settings
// systems, protocol fields, query parameters. It is also exactly the
// shape of an interpreter's Value, but nothing here requires one.)
//
// Three bugs, three classic failure modes:
//   1. the accessors don't check the tag: config_as_number happily
//      "reads" a number out of a bool. Contract: return -1 on a tag
//      mismatch and leave *out untouched;
//   2. config_free frees the string payload NO MATTER what the tag
//      says — freeing bytes of a double reinterpreted as a pointer
//      (the demo does it; ASan names it precisely);
//   3. config_equal compares strings by POINTER, so two equal strings
//      in different buffers are "different" (function_pointers3 again,
//      inside a union this time).

#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

typedef enum {
    CFG_NULL,
    CFG_BOOL,
    CFG_NUMBER,
    CFG_STRING,
} ConfigType;

typedef struct {
    ConfigType type;
    union {
        bool boolean;
        double number;
        char *string; /* owned, heap-allocated */
    } as;
} ConfigValue;

ConfigValue config_null(void) {
    return (ConfigValue){.type = CFG_NULL};
}

ConfigValue config_bool(bool b) {
    return (ConfigValue){.type = CFG_BOOL, .as.boolean = b};
}

ConfigValue config_number(double n) {
    return (ConfigValue){.type = CFG_NUMBER, .as.number = n};
}

// Owns a copy of s; returns a CFG_NULL value if allocation fails.
ConfigValue config_string(const char *s) {
    size_t len = strlen(s);
    char *copy = malloc(len + 1);
    if (!copy) {
        return config_null();
    }
    memcpy(copy, s, len + 1);
    return (ConfigValue){.type = CFG_STRING, .as.string = copy};
}

// Returns 0 and stores the number, or -1 if v is not a number
// (*out untouched in that case).
int config_as_number(const ConfigValue *v, double *out) {
    // BUG: no tag check — "reads" a number out of anything.
    *out = v->as.number;
    return 0;
}

// Returns 0 and stores the bool, or -1 if v is not a bool
// (*out untouched in that case).
int config_as_bool(const ConfigValue *v, bool *out) {
    // BUG: same missing tag check.
    *out = v->as.boolean;
    return 0;
}

// Returns the string, or NULL if v is not a string.
const char *config_as_string(const ConfigValue *v) {
    return v->type == CFG_STRING ? v->as.string : NULL;
}

// Values are equal when their type AND contents match.
bool config_equal(const ConfigValue *a, const ConfigValue *b) {
    if (a->type != b->type) {
        return false;
    }
    switch (a->type) {
    case CFG_NULL:
        return true;
    case CFG_BOOL:
        return a->as.boolean == b->as.boolean;
    case CFG_NUMBER:
        return a->as.number == b->as.number;
    case CFG_STRING:
        // BUG: compares the pointers, not the characters.
        return a->as.string == b->as.string;
    }
    return false;
}

// Releases whatever the value owns and resets it to CFG_NULL.
void config_free(ConfigValue *v) {
    // BUG: frees the string member even when the tag says the union
    // holds a bool or a double — free() on reinterpreted bytes.
    free(v->as.string);
    v->type = CFG_NULL;
}

#ifndef TEST
int main(void) {
    ConfigValue name = config_string("timeout");
    ConfigValue limit = config_number(42.0);

    printf("name:  %s\n", config_as_string(&name));
    double d = 0;
    if (config_as_number(&limit, &d) == 0) {
        printf("limit: %g\n", d);
    }

    // Freeing must respect the tag: `limit` owns NO heap memory, and
    // freeing the bytes of 42.0 as if they were a pointer is exactly
    // the kind of bug AddressSanitizer exists to name.
    config_free(&limit);
    config_free(&name);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_accessors_check_the_tag) {
    ConfigValue b = config_bool(true);
    double d = 99.5;
    ASSERT_EQ(config_as_number(&b, &d), -1);
    ASSERT(d == 99.5); /* untouched on mismatch */

    ConfigValue n = config_number(3.5);
    bool flag = true;
    ASSERT_EQ(config_as_bool(&n, &flag), -1);
    ASSERT(flag == true); /* untouched on mismatch */
    ASSERT(config_as_string(&n) == NULL);
}

TEST(test_accessors_happy_path) {
    ConfigValue n = config_number(2.5);
    double d = 0;
    ASSERT_EQ(config_as_number(&n, &d), 0);
    ASSERT(d == 2.5);

    ConfigValue b = config_bool(false);
    bool flag = true;
    ASSERT_EQ(config_as_bool(&b, &flag), 0);
    ASSERT(flag == false);
}

TEST(test_equal_strings_by_content) {
    ConfigValue a = config_string("hello");
    ConfigValue b = config_string("hello");
    ConfigValue c = config_string("world");
    ASSERT(config_equal(&a, &b)); /* different buffers, same text */
    ASSERT(!config_equal(&a, &c));
    config_free(&a);
    config_free(&b);
    config_free(&c);
}

TEST(test_equal_mixed_types) {
    ConfigValue n = config_number(1.0);
    ConfigValue b = config_bool(true);
    ConfigValue nil = config_null();
    ASSERT(!config_equal(&n, &b));
    ASSERT(!config_equal(&b, &nil));
    ASSERT(config_equal(&nil, &nil));
}

TEST(test_free_resets_to_null) {
    ConfigValue s = config_string("gone");
    config_free(&s);
    ASSERT_EQ(s.type, CFG_NULL);
    /* freeing a CFG_NULL (or twice) must be harmless */
    config_free(&s);
    ASSERT_EQ(s.type, CFG_NULL);
}

int main(void) {
    RUN_TEST(test_accessors_check_the_tag);
    RUN_TEST(test_accessors_happy_path);
    RUN_TEST(test_equal_strings_by_content);
    RUN_TEST(test_equal_mixed_types);
    RUN_TEST(test_free_resets_to_null);
    TEST_REPORT();
}
#endif
