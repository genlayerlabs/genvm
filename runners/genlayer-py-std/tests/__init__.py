from pythonfuzz import fuzzer


def do_fuzzing(target, runs=100_000):
	fuzz = fuzzer.Fuzzer(target, runs=runs)
	try:
		fuzz.start()
	except SystemExit as e:
		assert e.code == 0

	for buf in fuzz._corpus._inputs:
		target(buf)  # otherwise coverage is not counted
