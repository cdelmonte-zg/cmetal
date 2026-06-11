// ub_lab5.c - Reading uninitialized memory
//
// Reading an uninitialized variable is UNDEFINED BEHAVIOR in C.
// malloc() returns uninitialized memory, so every field of a malloc'd
// struct must be explicitly initialized before being read.
//
// This often "works" because the OS may zero-fill pages, but it is
// still UB and sanitizers will catch it.
//
// Fix: initialize all fields after malloc, or use calloc().

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char name[32];
    int age;
    int score;
    int active;
} Student;

// Creates a new student. Only sets the name and age fields.
// BUG: `score` and `active` are left uninitialized!
// TODO: Initialize ALL fields so that no read is UB.
Student *student_new(const char *name, int age) {
    Student *s = malloc(sizeof(Student));
    if (!s) return NULL;

    /* Only two fields are initialized -- the rest are garbage */
    size_t len = strlen(name);
    if (len >= sizeof(s->name)) {
        len = sizeof(s->name) - 1;
    }
    memcpy(s->name, name, len);
    s->name[len] = '\0';

    s->age = age;

    /* BUG: s->score and s->active are uninitialized */

    return s;
}

// Prints a student summary. Reads ALL fields, including the
// uninitialized ones.
void student_print(const Student *s) {
    printf("Name: %s, Age: %d, Score: %d, Active: %s\n",
           s->name, s->age, s->score,
           s->active ? "yes" : "no");
}

// Returns the student's score if they are active, else 0.
// Reads both `score` and `active`, both of which may be uninitialized.
int student_effective_score(const Student *s) {
    if (s->active) {
        return s->score;
    }
    return 0;
}

void student_free(Student *s) {
    free(s);
}

#ifndef TEST
int main(void) {
    Student *s = student_new("Alice", 20);
    if (!s) return 1;

    student_print(s);
    printf("Effective score: %d\n", student_effective_score(s));

    student_free(s);
    return 0;
}
#else
#include "clings_test.h"

/* Plant a nonzero pattern in freed heap memory so the next malloc() of a
   Student hands back garbage instead of fresh zero-filled pages from the
   OS. Without this, uninitialized fields would "happen" to be 0 and the
   tests below would pass by luck. */
static void dirty_heap(void) {
    Student *p = malloc(sizeof(Student));
    if (p) {
        memset(p, 0xAA, sizeof(*p));
        free(p);
    }
}

TEST(test_create) {
    Student *s = student_new("Bob", 25);
    ASSERT(s != NULL);
    ASSERT_STR_EQ(s->name, "Bob");
    ASSERT_EQ(s->age, 25);
    student_free(s);
}

TEST(test_default_score) {
    dirty_heap();
    Student *s = student_new("Charlie", 30);
    ASSERT(s != NULL);
    /* After creation, score should be 0 (not garbage) */
    ASSERT_EQ(s->score, 0);
    student_free(s);
}

TEST(test_default_active) {
    dirty_heap();
    Student *s = student_new("Diana", 22);
    ASSERT(s != NULL);
    /* After creation, active should be 0 (not garbage) */
    ASSERT_EQ(s->active, 0);
    student_free(s);
}

TEST(test_effective_score_inactive) {
    dirty_heap();
    Student *s = student_new("Eve", 19);
    ASSERT(s != NULL);
    /* Inactive student should have effective score 0 */
    ASSERT_EQ(student_effective_score(s), 0);
    student_free(s);
}

TEST(test_effective_score_active) {
    Student *s = student_new("Frank", 21);
    ASSERT(s != NULL);
    s->active = 1;
    s->score = 95;
    ASSERT_EQ(student_effective_score(s), 95);
    student_free(s);
}

TEST(test_long_name_truncated) {
    Student *s = student_new("Abcdefghijklmnopqrstuvwxyz1234567890", 18);
    ASSERT(s != NULL);
    ASSERT_EQ(strlen(s->name), 31u);
    student_free(s);
}

int main(void) {
    RUN_TEST(test_create);
    RUN_TEST(test_default_score);
    RUN_TEST(test_default_active);
    RUN_TEST(test_effective_score_inactive);
    RUN_TEST(test_effective_score_active);
    RUN_TEST(test_long_name_truncated);
    TEST_REPORT();
}
#endif
