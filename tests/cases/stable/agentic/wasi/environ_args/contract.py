# { "Depends": "py-genlayer:test" }
import genlayer as gl
import os
import sys


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		env_vars = dict(os.environ)
		for k in sorted(env_vars.keys()):
			res.append(f'env_{k}={env_vars[k]}')

		res.append(f'sys_argv={sys.argv}')

		for i, p in enumerate(sys.path):
			res.append(f'sys_path_{i}={p}')

		res.append(f'sys_platform={sys.platform}')
		res.append(f'sys_version={sys.version}')
		res.append(f'sys_byteorder={sys.byteorder}')
		res.append(f'sys_maxsize={sys.maxsize}')
		res.append(f'os_name={os.name}')

		try:
			cwd = os.getcwd()
			res.append(f'os_cwd={cwd}')
		except Exception as e:
			res.append(f'os_cwd_error={e}')

		try:
			pid = os.getpid()
			res.append(f'os_pid={pid}')
		except Exception as e:
			res.append(f'os_pid_error={e}')

		gl.vm.UserError.immediate(res)
