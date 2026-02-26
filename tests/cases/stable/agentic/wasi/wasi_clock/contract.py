# { "Depends": "py-genlayer:test" }
from genlayer import *
import time


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: time.time() - should use WASI clock_time_get
		t = time.time()
		print(f'time_time={t}')

		# Test 2: time.time_ns()
		t_ns = time.time_ns()
		print(f'time_ns={t_ns}')

		# Test 3: time.monotonic() - might differ
		try:
			m = time.monotonic()
			print(f'monotonic={m}')
		except Exception as e:
			print(f'monotonic_error={e}')

		# Test 4: time.monotonic_ns()
		try:
			m_ns = time.monotonic_ns()
			print(f'monotonic_ns={m_ns}')
		except Exception as e:
			print(f'monotonic_ns_error={e}')

		# Test 5: time.perf_counter() - likely non-deterministic
		try:
			p = time.perf_counter()
			print(f'perf_counter={p}')
		except Exception as e:
			print(f'perf_counter_error={e}')

		# Test 6: time.process_time()
		try:
			pt = time.process_time()
			print(f'process_time={pt}')
		except Exception as e:
			print(f'process_time_error={e}')

		# Test 7: time.clock_gettime with various clock IDs
		for clock_id_name in [
			'CLOCK_REALTIME',
			'CLOCK_MONOTONIC',
			'CLOCK_PROCESS_CPUTIME_ID',
			'CLOCK_THREAD_CPUTIME_ID',
		]:
			try:
				cid = getattr(time, clock_id_name, None)
				if cid is not None:
					val = time.clock_gettime(cid)
					print(f'clock_gettime_{clock_id_name}={val}')
			except Exception as e:
				print(f'clock_gettime_{clock_id_name}_error={e}')
