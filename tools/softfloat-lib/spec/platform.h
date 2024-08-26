#define LITTLEENDIAN 1

#ifdef __GNUC_STDC_INLINE__
#define INLINE inline
#else
#define INLINE extern inline
#endif

#define SOFTFLOAT_BUILTIN_CLZ 0
#define SOFTFLOAT_INTRINSIC_INT128 0
