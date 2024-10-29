#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifdef __cplusplus
extern "C"
{
#endif

static const char* error = NULL;

struct lib_t
{
	char* name;
	int32_t len;
};

int
dlclose(void* library)
{
	if (library == NULL) {
		return 0;
	}
	struct lib_t* lib = library;
	free(lib->name);
	free(lib);
	return 0;
}

char*
dlerror(void)
{
	char const* var = error;
	error           = NULL;
	return (char*)var;
}

void*
dlopen(const char* name, int flags)
{
	struct lib_t* lib = malloc(sizeof(struct lib_t));
	if (lib == NULL) {
		error = "OOM";
		return NULL;
	}
	int32_t lib_name_len = strlen(name);
	char* lib_name       = malloc(lib_name_len);
	if (lib_name == NULL) {
		free(lib);
		error = "OOM";
		return NULL;
	}
	memcpy(lib_name, name, lib_name_len);
	lib->len  = lib_name_len;
	lib->name = lib_name;
	return lib;
}

void*
gl_dlsym(
	const char* lib_name,
	int32_t lib_name_len,
	const char* fn_name,
	int32_t fn_name_len
) __attribute__((import_module("genlayer_dl"), import_name("dlsym")));

void*
dlsym(void* library, const char* name)
{
	if (library == NULL) {
		error = "library == NULL";
		return NULL;
	}
	struct lib_t* lib = library;
	return gl_dlsym(lib->name, lib->len, name, strlen(name));
}

#ifdef __cplusplus
}
#endif
