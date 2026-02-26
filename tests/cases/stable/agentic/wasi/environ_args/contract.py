# { "Depends": "py-genlayer:test" }
from genlayer import *
import os
import sys


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: All environment variables
		env_vars = dict(os.environ)
		for k in sorted(env_vars.keys()):
			print(f'env_{k}={env_vars[k]}')

		# Test 2: sys.argv
		print(f'sys_argv={sys.argv}')

		# Test 3: sys.path
		for i, p in enumerate(sys.path):
			print(f'sys_path_{i}={p}')

		# Test 4: sys.platform
		print(f'sys_platform={sys.platform}')

		# Test 5: sys.version
		print(f'sys_version={sys.version}')

		# Test 6: sys.byteorder
		print(f'sys_byteorder={sys.byteorder}')

		# Test 7: sys.maxsize
		print(f'sys_maxsize={sys.maxsize}')

		# Test 8: os.name
		print(f'os_name={os.name}')

		# Test 9: os.getcwd()
		try:
			cwd = os.getcwd()
			print(f'os_cwd={cwd}')
		except Exception as e:
			print(f'os_cwd_error={e}')

		# Test 10: os.getpid() - should this be deterministic?
		try:
			pid = os.getpid()
			print(f'os_pid={pid}')
		except Exception as e:
			print(f'os_pid_error={e}')
