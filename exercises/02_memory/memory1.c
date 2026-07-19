// memory1.c - Dynamic memory management
//
// This program builds a dynamic array of strings (a "string list").
// There are memory management bugs: leaks, missing NULL checks, or wrong
// free order. Fix them all!

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char **items;
    int count;
    int capacity;
} StringList;

// Helper: duplicate a string using malloc + strcpy
char *my_strdup(const char *s) {
    size_t len = strlen(s) + 1;
    char *copy = malloc(len);
    if (copy) {
        memcpy(copy, s, len);
    }
    return copy;
}

StringList *stringlist_new(int capacity) {
    StringList *list = malloc(sizeof(StringList));
    // TODO: Check for allocation failure (what if malloc returns NULL?)
    list->items = malloc(sizeof(char *) * (size_t)capacity);
    list->count = 0;
    list->capacity = capacity;
    return list;
}

void stringlist_add(StringList *list, const char *str) {
    if (list->count >= list->capacity) {
        list->capacity *= 2;
        list->items = realloc(list->items, sizeof(char *) * (size_t)list->capacity);
    }
    // Note: my_strdup allocates memory. Keep that in mind for cleanup!
    list->items[list->count] = my_strdup(str);
    list->count++;
}

void stringlist_print(const StringList *list) {
    for (int i = 0; i < list->count; i++) {
        printf("  [%d] %s\n", i, list->items[i]);
    }
}

void stringlist_free(StringList *list) {
    // BUG: This doesn't free the individual strings!
    // Each string was allocated by my_strdup() and needs its own free().
    free(list->items);
    free(list);
}

#ifndef TEST
int main(void) {
    StringList *list = stringlist_new(2);
    stringlist_add(list, "hello");
    stringlist_add(list, "advanced");
    stringlist_add(list, "C");

    stringlist_print(list);
    stringlist_free(list);

    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_create) {
    StringList *list = stringlist_new(2);
    ASSERT(list != NULL);
    ASSERT(list->items != NULL);
    stringlist_free(list);
}

TEST(test_add_and_grow) {
    StringList *list = stringlist_new(2);
    stringlist_add(list, "alpha");
    stringlist_add(list, "beta");
    ASSERT_EQ(list->count, 2);

    // This triggers realloc (capacity was 2)
    stringlist_add(list, "gamma");
    ASSERT_EQ(list->count, 3);

    ASSERT_STR_EQ(list->items[0], "alpha");
    ASSERT_STR_EQ(list->items[1], "beta");
    ASSERT_STR_EQ(list->items[2], "gamma");

    stringlist_free(list);
}

int main(void) {
    RUN_TEST(test_create);
    RUN_TEST(test_add_and_grow);
    TEST_REPORT();
}
#endif
