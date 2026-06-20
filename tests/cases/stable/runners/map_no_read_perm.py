# { "Depends": "py-genlayer:test" }
from genlayer.vm import map_file

# permissions for this case omit `r` (read_storage), so map_file must be refused
try:
	map_file('contract', 'file', '/mapped')
	print('mapped')
except Exception as e:
	print(f'forbidden: {type(e).__name__}')
exit(0)
