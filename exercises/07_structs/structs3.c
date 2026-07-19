// structs3.c - Linked list operations
//
// A singly linked list is the classic self-referential struct in C.
// Each node holds a value and a pointer to the next node.
//
// Implement basic linked list operations and fix the bugs.

#include <stdio.h>
#include <stdlib.h>

struct node {
    int value;
    struct node *next;
};

// Push a new node to the front of the list.
// Returns the new head.
//
// BUG: The new node's next pointer is not set correctly.
// The new node should point to the old head, but it doesn't.
//
// TODO: Fix so the new node links to the existing list.
struct node *list_push_front(struct node *head, int value) {
    struct node *new_node = malloc(sizeof(struct node));
    if (!new_node) return head;
    new_node->value = value;
    new_node->next = NULL;  // BUG: should point to head, not NULL
    return new_node;
}

// Remove and return the value of the front node.
// Updates *head_ptr to point to the new head.
// Returns -1 if the list is empty.
int list_pop_front(struct node **head_ptr) {
    if (*head_ptr == NULL) return -1;
    struct node *old_head = *head_ptr;
    int value = old_head->value;
    *head_ptr = old_head->next;
    free(old_head);
    return value;
}

// Find the first node with the given value.
// Returns a pointer to that node, or NULL if not found.
struct node *list_find(struct node *head, int value) {
    struct node *current = head;
    while (current != NULL) {
        if (current->value == value) {
            return current;
        }
        current = current->next;
    }
    return NULL;
}

// Free all nodes in the list.
//
// BUG: This function doesn't free any nodes — it just leaks
// all the memory! You need to walk the list, freeing each node.
//
// TODO: Traverse the list and free each node. Be careful:
// save node->next BEFORE calling free(node), because you can't
// access a node after it's been freed.
void list_free(struct node *head) {
    // BUG: does nothing — all nodes are leaked!
    (void)head;
}

#ifndef TEST
int main(void) {
    struct node *list = NULL;
    list = list_push_front(list, 10);
    list = list_push_front(list, 20);
    list = list_push_front(list, 30);

    struct node *cur = list;
    while (cur) {
        printf("%d -> ", cur->value);
        cur = cur->next;
    }
    printf("NULL\n");

    list_free(list);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_push_front) {
    struct node *list = NULL;
    list = list_push_front(list, 10);
    list = list_push_front(list, 20);
    list = list_push_front(list, 30);

    // List should be: 30 -> 20 -> 10 -> NULL
    ASSERT(list != NULL);
    ASSERT_EQ(list->value, 30);
    ASSERT(list->next != NULL);
    ASSERT_EQ(list->next->value, 20);
    ASSERT(list->next->next != NULL);
    ASSERT_EQ(list->next->next->value, 10);
    ASSERT(list->next->next->next == NULL);

    list_free(list);
}

TEST(test_pop_front) {
    struct node *list = NULL;
    list = list_push_front(list, 10);
    list = list_push_front(list, 20);

    ASSERT_EQ(list_pop_front(&list), 20);
    ASSERT_EQ(list_pop_front(&list), 10);
    ASSERT_EQ(list_pop_front(&list), -1);
    ASSERT(list == NULL);
}

TEST(test_find) {
    struct node *list = NULL;
    list = list_push_front(list, 10);
    list = list_push_front(list, 20);
    list = list_push_front(list, 30);

    struct node *found = list_find(list, 20);
    ASSERT(found != NULL);
    ASSERT_EQ(found->value, 20);

    ASSERT(list_find(list, 99) == NULL);

    list_free(list);
}

TEST(test_empty_list) {
    struct node *list = NULL;
    ASSERT(list_find(list, 42) == NULL);
    ASSERT_EQ(list_pop_front(&list), -1);
}

TEST(test_single_element) {
    struct node *list = NULL;
    list = list_push_front(list, 7);
    ASSERT_EQ(list->value, 7);
    ASSERT(list->next == NULL);
    ASSERT_EQ(list_pop_front(&list), 7);
    ASSERT(list == NULL);
}

int main(void) {
    RUN_TEST(test_push_front);
    RUN_TEST(test_pop_front);
    RUN_TEST(test_find);
    RUN_TEST(test_empty_list);
    RUN_TEST(test_single_element);
    TEST_REPORT();
}
#endif
