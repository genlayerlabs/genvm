KNOWN_CONTRACT = None


def run(mod):
	contract = getattr(mod, '__KNOWN_CONTRACT')
	from ._runner import run as r

	r(contract)
