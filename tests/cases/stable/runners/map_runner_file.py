# { "Depends": "py-genlayer:test" }
from genlayer.vm import register_runner, map_file

code = b'# { "Depends": "py-genlayer:test" }\nmarker-12345\n'
rid = register_runner(code)
map_file(rid, 'file', '/mapped.txt')
with open('/mapped.txt', 'rb') as f:
	print(f.read() == code)
exit(0)
