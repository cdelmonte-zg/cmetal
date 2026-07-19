// tagged4.c - A header struct as the first member: polymorphism in C
//
// The other way to do variants (besides the tagged union): give every
// concrete struct a common HEADER as its FIRST member. C11 guarantees
// there is no padding before a struct's first member (6.7.2.1p15): a
// pointer to the struct and a pointer to its first member are the same
// address, so upcasting a Paragraph* to Node* is well-defined — AS LONG
// AS the header really is first. That single word, "first", is the
// entire contract of this idiom.
//
// (Editorial note: heterogeneous nodes behind a common header are any
// document tree, scene graph or plugin registry — CPython's PyObject
// and the sockaddr family work exactly like this. So does an
// interpreter's object header, but nothing here requires one.)
//
// Two bugs:
//   1. Image declares its header LAST, with a comment claiming the
//      order doesn't matter. With the header at a nonzero offset,
//      (Node *)img reads pixel counts as a type tag;
//   2. node_text_length dispatches on the tag correctly but then
//      downcasts to the WRONG concrete type (copy-paste), reading an
//      Image as if it were a Paragraph.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef enum {
    NODE_PARAGRAPH,
    NODE_IMAGE,
} NodeType;

// The common header: every concrete node embeds this as its FIRST member.
typedef struct {
    NodeType type;
} Node;

typedef struct {
    Node base; /* first member: (Node *)&p and &p.base are the same */
    char *text; /* owned */
} Paragraph;

typedef struct {
    int width;
    int height;
    // BUG: "the position of the header doesn't matter" — it does. It
    // is the only thing that makes (Node *)img valid: the header must
    // come FIRST.
    Node base;
} Image;

// Allocates a paragraph owning a copy of text; NULL on failure.
Node *paragraph_new(const char *text) {
    Paragraph *p = malloc(sizeof(Paragraph));
    if (!p) {
        return NULL;
    }
    size_t len = strlen(text);
    p->text = malloc(len + 1);
    if (!p->text) {
        free(p);
        return NULL;
    }
    memcpy(p->text, text, len + 1);
    p->base.type = NODE_PARAGRAPH;
    return (Node *)p;
}

// Allocates an image node; NULL on failure.
Node *image_new(int width, int height) {
    Image *img = malloc(sizeof(Image));
    if (!img) {
        return NULL;
    }
    img->width = width;
    img->height = height;
    img->base.type = NODE_IMAGE;
    return (Node *)img;
}

NodeType node_type(const Node *n) {
    return n->type;
}

// Text length of the node: paragraphs report their text, images 0.
size_t node_text_length(const Node *n) {
    switch (n->type) {
    case NODE_PARAGRAPH:
        return strlen(((const Paragraph *)n)->text);
    case NODE_IMAGE:
        // BUG: copy-paste — an Image has no text member; this reads
        // pixel counts as a char pointer and calls strlen on it.
        return strlen(((const Paragraph *)n)->text);
    }
    return 0;
}

int image_width(const Node *n) {
    return n->type == NODE_IMAGE ? ((const Image *)n)->width : -1;
}

// Releases the node and everything it owns.
void node_free(Node *n) {
    if (!n) {
        return;
    }
    switch (n->type) {
    case NODE_PARAGRAPH:
        free(((Paragraph *)n)->text);
        break;
    case NODE_IMAGE:
        break;
    }
    free(n);
}

#ifndef TEST
int main(void) {
    Node *nodes[2];
    nodes[0] = paragraph_new("hello, world");
    nodes[1] = image_new(640, 480);
    if (!nodes[0] || !nodes[1]) {
        return 1;
    }

    for (int i = 0; i < 2; i++) {
        printf("node %d: type=%d text_length=%zu\n", i, node_type(nodes[i]),
               node_text_length(nodes[i]));
    }
    printf("image width: %d\n", image_width(nodes[1]));

    node_free(nodes[0]);
    node_free(nodes[1]);
    return 0;
}
#else
#include "cmetal_test.h"

TEST(test_upcast_preserves_the_tag) {
    Node *p = paragraph_new("x");
    Node *img = image_new(640, 480);
    ASSERT(p != NULL);
    ASSERT(img != NULL);
    ASSERT_EQ(node_type(p), NODE_PARAGRAPH);
    /* With the header anywhere but FIRST, this reads 640, not the tag. */
    ASSERT_EQ(node_type(img), NODE_IMAGE);
    node_free(p);
    node_free(img);
}

TEST(test_downcast_roundtrip) {
    Node *n = paragraph_new("through the base and back");
    ASSERT(n != NULL);
    Paragraph *p = (Paragraph *)n;
    ASSERT_STR_EQ(p->text, "through the base and back");
    node_free(n);
}

TEST(test_dispatch_uses_the_right_type) {
    Node *p = paragraph_new("hello");
    Node *img = image_new(2, 3);
    ASSERT(p != NULL);
    ASSERT(img != NULL);
    ASSERT_EQ(node_text_length(p), 5u);
    ASSERT_EQ(node_text_length(img), 0u); /* an image has no text */
    ASSERT_EQ(image_width(img), 2);
    ASSERT_EQ(image_width(p), -1);
    node_free(p);
    node_free(img);
}

int main(void) {
    RUN_TEST(test_upcast_preserves_the_tag);
    RUN_TEST(test_downcast_roundtrip);
    RUN_TEST(test_dispatch_uses_the_right_type);
    TEST_REPORT();
}
#endif
