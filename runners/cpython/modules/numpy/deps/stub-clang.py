#!/usr/bin/env python3

import subprocess, sys

args = [
	arg
	for arg in sys.argv[1:]
	if arg
	not in [
		'-Wl,--start-group',
		'-Wl,--end-group',
		'-Wl,--as-needed',
		'-Wl,--allow-shlib-undefined',
		'-fPIC',
	]
	and not arg.startswith('-I/nix/store/')
]

inp = ''
if '-shared' in args:
	print('faking shared library')
	cmd = ['ar', '-M']
	dup = list(args)
	dup.reverse()
	o_file = None
	inputs = []
	while len(dup) > 0:
		arg = dup.pop()
		if arg in ['-shared', '-fPIC']:
			continue
		if arg == '-o':
			o_file = dup.pop()
		elif arg.startswith('-l'):
			continue
		elif arg.startswith('-'):
			raise Exception('unknown arg ', arg)
		else:
			inputs.append(arg)
	inp_lines = [f'CREATE {o_file}']
	for x in inputs:
		if x.endswith('.a'):
			inp_lines.append(f'ADDLIB {x}')
		else:
			inp_lines.append(f'ADDMOD {x}')
	inp_lines.append('SAVE')
	inp_lines.append('END')
	inp = '\n'.join(inp_lines)
	print(inp)
else:
	cmd = ['/build/wasi-sdk/bin/clang'] + args
res = subprocess.run(cmd, check=False, text=True, capture_output=True, input=inp)
print(res.stdout, end='')
print(res.stderr, file=sys.stderr, end='')
exit(res.returncode)
