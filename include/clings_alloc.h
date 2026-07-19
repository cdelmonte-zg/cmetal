/*
 * clings_alloc.h — compatibility shim (the project was renamed cmetal).
 * Working copies created before the rename still use the old names.
 */
#ifndef CLINGS_ALLOC_COMPAT_H
#define CLINGS_ALLOC_COMPAT_H

#include "cmetal_alloc.h"

#define CLINGS_MALLOC  CMETAL_MALLOC
#define CLINGS_REALLOC CMETAL_REALLOC
#ifdef TEST
#define clings_fail_next_alloc cmetal_fail_next_alloc
#define clings_alloc_reset     cmetal_alloc_reset
#endif

#endif
