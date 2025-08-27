#!/usr/bin/env python3
import sys

files = sys.argv[1:]
files.sort()
exit_code = 0
for file in files:
	if file.endswith('.rs'):
		continue
	if file.endswith('.py'):
		continue
	if 'runners/py-libs' in file:
		continue
	with open(file, 'rt') as f:
		try:
			s = f.readline()
		except:
			continue
		if not s.startswith('#!'):
			continue
		if not s.startswith('#!/usr/bin/env'):
			print(f'invalid shebang in {file}: {s}')
			exit_code = 1

exit(exit_code)
