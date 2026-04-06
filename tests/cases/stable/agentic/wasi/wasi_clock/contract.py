# { "Depends": "py-genlayer:test" }
import genlayer as gl
import time


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		t = time.time()
		res.append(f'time_time={t}')

		t_ns = time.time_ns()
		res.append(f'time_ns={t_ns}')

		try:
			m = time.monotonic()
			res.append(f'monotonic={m}')
		except Exception as e:
			res.append(f'monotonic_error={e}')

		try:
			m_ns = time.monotonic_ns()
			res.append(f'monotonic_ns={m_ns}')
		except Exception as e:
			res.append(f'monotonic_ns_error={e}')

		try:
			p = time.perf_counter()
			res.append(f'perf_counter={p}')
		except Exception as e:
			res.append(f'perf_counter_error={e}')

		try:
			pt = time.process_time()
			res.append(f'process_time={pt}')
		except Exception as e:
			res.append(f'process_time_error={e}')

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
					res.append(f'clock_gettime_{clock_id_name}={val}')
			except Exception as e:
				res.append(f'clock_gettime_{clock_id_name}_error={e}')

		gl.vm.UserError.immediate(res)
