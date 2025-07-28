import numpy as np

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

	for testcase in src_dir.parent.joinpath('inputs', name).iterdir():

		def cur_test(testcase=testcase):
			fun(testcase.read_bytes())

		testname = 'test_' + name + '_' + testcase.name.replace('.', '_')

		cur_test.__name__ = testname

		globals()[testname] = cur_test
