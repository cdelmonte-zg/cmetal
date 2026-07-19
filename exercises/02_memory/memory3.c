// memory3.c - Ownership-destroying functions and the double pointer
//
// This program implements a 2D matrix using dynamically allocated arrays.
//
// matrix_destroy takes a Matrix ** — a pointer to the CALLER's pointer —
// so that, besides freeing everything, it can set the caller's pointer
// to NULL. A destroy function that takes a plain Matrix * can free the
// memory, but it leaves the caller holding a dangling pointer; nulling
// fields inside a struct that is itself about to be freed helps nobody.
// After matrix_destroy(&m), m is NULL, and a second matrix_destroy(&m)
// is a harmless no-op instead of a double free.
//
// Bugs to fix: matrix_destroy leaks every row (only the row-pointer
// array is freed), and its contract is wrong — it takes Matrix *, so
// it CANNOT null the caller's pointer, which stays dangling, and a
// second call double-frees. Part of the fix is changing the signature.

#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int **data;
    int rows;
    int cols;
} Matrix;

Matrix *matrix_create(int rows, int cols) {
    Matrix *m = malloc(sizeof(Matrix));
    if (!m) return NULL;
    m->rows = rows;
    m->cols = cols;

    if (rows == 0) {
        m->data = NULL;
        return m;
    }

    m->data = malloc(sizeof(int *) * (size_t)rows);
    if (!m->data) {
        free(m);
        return NULL;
    }

    for (int i = 0; i < rows; i++) {
        m->data[i] = malloc(sizeof(int) * (size_t)cols);
        if (!m->data[i]) {
            for (int j = 0; j < i; j++) {
                free(m->data[j]);
            }
            free(m->data);
            free(m);
            return NULL;
        }
        for (int c = 0; c < cols; c++) {
            m->data[i][c] = 0;
        }
    }

    return m;
}

void matrix_set(Matrix *m, int r, int c, int val) {
    if (r >= 0 && r < m->rows && c >= 0 && c < m->cols) {
        m->data[r][c] = val;
    }
}

int matrix_get(const Matrix *m, int r, int c) {
    if (r >= 0 && r < m->rows && c >= 0 && c < m->cols) {
        return m->data[r][c];
    }
    return -1;
}

// TODO: change the contract to
//     void matrix_destroy(Matrix **m);
// then free every row, the row-pointer array, and *m itself, and
// finally set *m = NULL. Note: a changed signature ripples to every
// call site — the demo and the tests below already call
// matrix_destroy(&m), so this file won't even compile until the
// function catches up with its callers.
void matrix_destroy(Matrix *m) {
    if (!m) return;
    // BUG: This only frees the array of row pointers, but not the
    // individual row arrays! Each m->data[i] was allocated with
    // malloc and needs its own free().
    free(m->data);
    free(m);
    // BUG: The caller's pointer still holds the freed address, and
    // through a Matrix * this function has no way to fix that — only
    // a Matrix ** lets it reach the caller's pointer and null it.
}

#ifndef TEST
int main(void) {
    Matrix *m = matrix_create(3, 3);
    if (!m) {
        printf("Allocation failed!\n");
        return 1;
    }

    matrix_set(m, 0, 0, 1);
    matrix_set(m, 1, 1, 5);
    matrix_set(m, 2, 2, 9);

    printf("Matrix 3x3:\n");
    for (int r = 0; r < m->rows; r++) {
        for (int c = 0; c < m->cols; c++) {
            printf(" %3d", matrix_get(m, r, c));
        }
        printf("\n");
    }

    matrix_destroy(&m);
    printf("After destroy, m is %s\n", m == NULL ? "NULL" : "dangling!");
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_create) {
    Matrix *m = matrix_create(3, 3);
    ASSERT(m != NULL);
    ASSERT(m->data != NULL);
    ASSERT_EQ(m->rows, 3);
    ASSERT_EQ(m->cols, 3);
    matrix_destroy(&m);
}

TEST(test_set_and_get) {
    Matrix *m = matrix_create(3, 3);
    matrix_set(m, 0, 0, 1);
    matrix_set(m, 1, 1, 5);
    matrix_set(m, 2, 2, 9);
    ASSERT_EQ(matrix_get(m, 0, 0), 1);
    ASSERT_EQ(matrix_get(m, 1, 1), 5);
    ASSERT_EQ(matrix_get(m, 2, 2), 9);
    // Unset cells should be zero-initialized
    ASSERT_EQ(matrix_get(m, 0, 1), 0);
    ASSERT_EQ(matrix_get(m, 1, 0), 0);
    matrix_destroy(&m);
}

TEST(test_bounds_check) {
    Matrix *m = matrix_create(3, 3);
    ASSERT_EQ(matrix_get(m, -1, 0), -1);
    ASSERT_EQ(matrix_get(m, 0, -1), -1);
    ASSERT_EQ(matrix_get(m, 3, 0), -1);
    ASSERT_EQ(matrix_get(m, 0, 3), -1);
    matrix_destroy(&m);
}

TEST(test_empty_matrix) {
    Matrix *m = matrix_create(0, 0);
    ASSERT(m != NULL);
    ASSERT_EQ(m->rows, 0);
    ASSERT_EQ(m->data, NULL);
    matrix_destroy(&m);
}

TEST(test_destroy_nulls_caller_pointer) {
    Matrix *m = matrix_create(2, 2);
    ASSERT(m != NULL);
    matrix_destroy(&m);
    ASSERT(m == NULL);
}

TEST(test_double_destroy_is_safe) {
    Matrix *m = matrix_create(2, 2);
    ASSERT(m != NULL);
    matrix_destroy(&m);
    matrix_destroy(&m);  /* *m is NULL now: must be a no-op */
    ASSERT(m == NULL);
    matrix_destroy(NULL);  /* a NULL handle must also be safe */
}

int main(void) {
    RUN_TEST(test_create);
    RUN_TEST(test_set_and_get);
    RUN_TEST(test_bounds_check);
    RUN_TEST(test_empty_matrix);
    RUN_TEST(test_destroy_nulls_caller_pointer);
    RUN_TEST(test_double_destroy_is_safe);
    TEST_REPORT();
}
#endif
