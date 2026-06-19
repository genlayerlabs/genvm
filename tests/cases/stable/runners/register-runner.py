# { "Depends": "py-genlayer:test" }

from genlayer.vm import register_runner

code = b'# { "Depends": "py-genlayer:test" }\nexit(0)\n'
print(register_runner(code))
exit(0)
