#include <stdint.h>

__attribute__((import_module("genlayer_sdk"))) uint32_t
storage_read(char const* slot, uint32_t index, char* buf, uint32_t buf_len);
__attribute__((import_module("genlayer_sdk"))) uint32_t
storage_write(
	char const* slot,
	int32_t index,
	char const* buf,
	uint32_t buf_len
);
__attribute__((import_module("genlayer_sdk"))) uint32_t
get_balance(char const* address, char* result);
__attribute__((import_module("genlayer_sdk"))) uint32_t
get_self_balance(char* result);
__attribute__((import_module("genlayer_sdk"))) uint32_t
gl_call(char const* request, uint32_t request_len, uint32_t* result_fd);

#define PY_SSIZE_T_CLEAN
#include <Python.h>

static char const* const error_codes[] = {
	"success", "overflow", "inval",     "fault",
	"ilseq",   "io",       "forbidden", "inbalance",
};

static const size_t error_codes_len =
	sizeof(error_codes) / sizeof(error_codes[0]);

static void
py_set_exc_from_result(uint32_t res)
{
	if (res == 0) {
		return;
	}

	PyErr_Format(
		PyExc_SystemError,
		"%d: %s",
		res,
		res < error_codes_len ? error_codes[res] : "<unknown>"
	);
}

static PyObject*
py_storage_read(PyObject* _self, PyObject* args)
{
	Py_buffer slot;
	uint32_t index;
	Py_buffer buffer;

	if (!PyArg_ParseTuple(args, "y*Iw*", &slot, &index, &buffer)) {
		return NULL;
	}

	uint32_t res = storage_read(slot.buf, index, (char*)buffer.buf, buffer.len);

	PyBuffer_Release(&slot);
	PyBuffer_Release(&buffer);

	if (res != 0) {
		py_set_exc_from_result(res);

		return NULL;
	}

	Py_RETURN_NONE;
}
static PyObject*
py_storage_write(PyObject* _self, PyObject* args)
{
	Py_buffer slot;
	uint32_t index;
	Py_buffer buffer;

	if (!PyArg_ParseTuple(args, "y*Iy*", &slot, &index, &buffer)) {
		return NULL;
	}

	uint32_t res =
		storage_write(slot.buf, index, (char const*)buffer.buf, buffer.len);

	PyBuffer_Release(&slot);
	PyBuffer_Release(&buffer);

	if (res != 0) {
		py_set_exc_from_result(res);

		return NULL;
	}

	Py_RETURN_NONE;
}

static PyObject*
py_get_balance(PyObject* _self, PyObject* args)
{
	Py_buffer address;

	if (!PyArg_ParseTuple(args, "y*", &address)) {
		return NULL;
	}

	char balance_result[32];

	uint32_t res = get_balance((char const*)address.buf, &balance_result[0]);

	PyBuffer_Release(&address);

	if (res != 0) {
		py_set_exc_from_result(res);

		return NULL;
	}

	return PyLong_FromUnsignedNativeBytes(
		&balance_result, 32, Py_ASNATIVEBYTES_LITTLE_ENDIAN
	);
}

static PyObject*
py_get_self_balance(PyObject* _self, PyObject* _args)
{
	char balance_result[32];

	uint32_t res = get_self_balance(&balance_result[0]);

	if (res != 0) {
		py_set_exc_from_result(res);

		return NULL;
	}

	return PyLong_FromUnsignedNativeBytes(
		&balance_result, 32, Py_ASNATIVEBYTES_LITTLE_ENDIAN
	);
}

static PyObject*
py_gl_call(PyObject* _self, PyObject* args)
{
	Py_buffer request;

	if (!PyArg_ParseTuple(args, "y*", &request)) {
		return NULL;
	}

	uint32_t fd;
	uint32_t res = gl_call((char const*)request.buf, request.len, &fd);

	PyBuffer_Release(&request);

	if (res != 0) {
		py_set_exc_from_result(res);

		return NULL;
	}

	return PyLong_FromLong(fd);
}

static PyMethodDef genlayer_module_methods[] = {
	{		 "storage_read",     py_storage_read, METH_VARARGS,   "" },
	{		"storage_write",    py_storage_write, METH_VARARGS,   "" },
	{			"get_balance",      py_get_balance, METH_VARARGS,   "" },
	{ "get_self_balance", py_get_self_balance, METH_VARARGS,   "" },
	{					"gl_call",          py_gl_call, METH_VARARGS,   "" },

	{							 NULL,								NULL,            0, NULL },
};

static struct PyModuleDef genlayer_module = {
	PyModuleDef_HEAD_INIT,
	"_genlayer_wasi",
	"documentation",
	0, // per-interpreter size
	genlayer_module_methods,
	NULL, // m_slots
	NULL, // m_traverse
	NULL, // m_clear
	NULL, // m_free
};

PyMODINIT_FUNC
PyInit__genlayer_wasi(void)
{
	return PyModule_Create(&genlayer_module);
}
