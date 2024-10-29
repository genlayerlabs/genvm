#include <ffi.h>
#include <stdlib.h>

void
ffi_call(ffi_cif* cif, void (*fn)(void), void* rvalue, void** avalue)
{
	abort();
}

ffi_status
ffi_prep_closure_loc(
	ffi_closure* closure,
	ffi_cif* cif,
	void (*fun)(ffi_cif*, void*, void**, void*),
	void* user_data,
	void* codeloc
)
{
	abort();
}

ffi_status
ffi_prep_cif_machdep(ffi_cif* cif)
{
	abort();
}

ffi_status
ffi_prep_cif_machdep_var(ffi_cif* cif, unsigned nfixedargs, unsigned ntotalargs)
{
	abort();
}
