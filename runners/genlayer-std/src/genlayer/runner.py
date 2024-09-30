def run(mod):
	if hasattr(mod, '__gsdk_self_run__') and mod.__gsdk_self_run__:
		return
	from ._runner import run as r
	r(mod)
