#include <pthread.h>
#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C"
{
#endif

    int pthread_create(
      pthread_t* __restrict,
      const pthread_attr_t* __restrict,
      void* (*)(void*),
      void* __restrict
    )
    {
	abort();
    }
    int pthread_detach(pthread_t)
    {
	abort();
    }

    int pthread_join(pthread_t, void**)
    {
	abort();
    }

#ifdef __cplusplus
}
#endif
