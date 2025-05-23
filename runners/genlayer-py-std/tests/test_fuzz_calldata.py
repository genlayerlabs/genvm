from pythonfuzz import fuzzer

import genlayer.py.calldata as calldata


def test_calldata_decoding():
	def fuzz_target(buf):
		try:
			decoded = calldata.decode(buf)
		except (calldata.DecodingError, UnicodeDecodeError):
			return
		got = calldata.encode(decoded)

		assert got == buf, f'decoded is `{decoded}`'

	from . import do_fuzzing

	do_fuzzing(fuzz_target)


def test_calldata_encoding():
	def fuzz_target(buf):
		class NotEnough(Exception):
			pass

		def fetch_buf(cnt: int) -> bytes:
			nonlocal buf
			if cnt > len(buf):
				raise NotEnough()
			ret = buf[:cnt]
			buf = buf[cnt:]
			return ret

		def create_atom():
			kind = fetch_buf(1)[0] % 6
			if kind == 0:
				return int.from_bytes(fetch_buf(7))
			if kind == 1:
				return True
			if kind == 2:
				return False
			if kind == 3:
				return None
			if kind == 4:
				return fetch_buf(fetch_buf(1)[0])
			if kind == 5:
				fetch_buf(fetch_buf(1)[0]).decode('utf-8')

		def create_any(depth):
			if depth == 0:
				return create_atom()
			kind = fetch_buf(1)[0] % 3
			if kind == 0:
				return create_atom()
			if kind == 1:
				le = fetch_buf(1)[0]
				lst = []
				for i in range(le):
					lst.append(create_any(depth - 1))
				return lst
			if kind == 1:
				le = fetch_buf(1)[0]
				lst = {}
				for i in range(le):
					k = fetch_buf(fetch_buf(1)[0]).decode('utf-8')
					v = create_any(depth - 1)
					lst[k] = v
				return lst

		try:
			depth = fetch_buf(1)[0] % 5
			created_obj = create_any(depth)
		except (UnicodeDecodeError, NotEnough):
			return

		encoded = '<unbound>'
		decoded = '<unbound>'
		try:
			encoded = calldata.encode(created_obj)
			decoded = calldata.decode(encoded)

			assert created_obj == decoded
		except Exception as e:
			raise Exception(
				f'failed base={created_obj} decoded={decoded} encoded={encoded}'
			) from e

	from . import do_fuzzing

	do_fuzzing(fuzz_target)
