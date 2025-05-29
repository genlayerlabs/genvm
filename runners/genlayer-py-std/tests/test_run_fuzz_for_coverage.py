from pathlib import Path

src_dir = Path(__file__).parent.parent.joinpath('fuzz', 'src')

for test in sorted(src_dir.iterdir()):
	name = test.name[:-3]
	print(test, name)

	src_py = test.read_text()
	new_globs = {}  # globals().copy()
	new_globs['__file__'] = str(test)
	exec(src_py, new_globs)
	fun = new_globs[name]

	def cur_test():
		print('I MADE IT TO TEST')
		for testcase in src_dir.parent.joinpath('inputs', name).iterdir():
			fun(testcase.read_bytes())

	cur_test.__name__ = 'test_' + name

	globals()[cur_test.__name__] = cur_test
