KNOWN_CONTRACT = None


def run(mod):
	if hasattr(mod, '__gl_self_run__') and mod.__gl_self_run__:
		return
	contract = getattr(mod, '__KNOWN_CONTRACT')
	from ._runner import run as r

	r(contract)
