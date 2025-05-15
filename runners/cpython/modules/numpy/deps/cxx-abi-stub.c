#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C"
{
#endif

void*
__cxa_allocate_exception(size_t thrown_size)
{
	abort();
}

void
__cxa_throw(void* thrown_exception, void* tinfo, void (*dest)(void*))
{
	abort();
}

#ifdef __cplusplus
}
#endif
