#define OPJ_PACKAGE_VERSION "2.5.3"

// #define _LARGEFILE_SOURCE
// #define _LARGE_FILES
#define _FILE_OFFSET_BITS 64
#define OPJ_HAVE_FSEEKO   1

/* find whether or not have <malloc.h> */
#define OPJ_HAVE_MALLOC_H
// #define OPJ_HAVE_MEMALIGN

#if !defined(_POSIX_C_SOURCE)
#if defined(OPJ_HAVE_FSEEKO) || defined(OPJ_HAVE_POSIX_MEMALIGN)
/* Get declarations of fseeko, ftello, posix_memalign. */
#define _POSIX_C_SOURCE 200112L
#endif
#endif
