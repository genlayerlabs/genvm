# { "Depends": "py-genlayer:test" }
from genlayer.vm import register_runner

# Registering the same runner repeatedly must be idempotent: it returns the same
# id every time and (see the executor unit test `register_custom_runner_consumes_once`)
# charges the memory limit only once, so a loop like this cannot exhaust it.
code = b'# { "Depends": "py-genlayer:test" }\nx\n'

ids = {register_runner(code) for _ in range(1000)}
assert len(ids) == 1, ids
print(next(iter(ids)))
exit(0)
