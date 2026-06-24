# { "Depends": "py-genlayer:test" }
from genlayer.vm import map_file

# mapping into /vm/ is forbidden; the runtime gl_call path must reject it just
# like the InitAction MapFile path does (see runners/map-vm). The rejection is a
# malformed_runner trap, so it terminates execution and is not catchable here.
map_file('contract', 'file', '/vm/evil.py')
exit(0)
