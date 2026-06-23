# { "Depends": "py-genlayer:test" }
from genlayer.vm import register_runner

# default permissions (rwscn) do NOT grant `register_runners` (u)
try:
	register_runner(b'# { "Depends": "py-genlayer:test" }\nx\n')
	print('registered')
except Exception as e:
	print(e)
exit(0)
