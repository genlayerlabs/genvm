import socket
import typing
import collections.abc
import asyncio
import os
import abc
import json

import aiohttp

from dataclasses import dataclass

from pathlib import Path

from . import host_fns
from . import public_abi

ACCOUNT_ADDR_SIZE = 20
SLOT_ID_SIZE = 32

from .logger import Logger, NoLogger


class HostException(Exception):
	def __init__(self, error_code: host_fns.Errors, message: str = ''):
		if error_code == host_fns.Errors.OK:
			raise ValueError('Error code cannot be OK')
		self.error_code = error_code
		super().__init__(message or f'GenVM error: {error_code}')


class DefaultEthTransactionData(typing.TypedDict):
	value: str


class DefaultTransactionData(typing.TypedDict):
	value: str
	on: str


class DeployDefaultTransactionData(DefaultTransactionData):
	salt_nonce: typing.NotRequired[str]


class IHost(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def loop_enter(self, cancellation: asyncio.Event) -> socket.socket: ...

	@abc.abstractmethod
	async def get_calldata(self, /) -> bytes: ...

	@abc.abstractmethod
	async def storage_read(
		self,
		mode: public_abi.StorageType,
		account: bytes,
		slot: bytes,
		index: int,
		le: int,
		/,
	) -> bytes: ...
	@abc.abstractmethod
	async def storage_write(
		self,
		slot: bytes,
		index: int,
		got: collections.abc.Buffer,
		/,
	) -> None: ...

	@abc.abstractmethod
	async def consume_result(
		self, type: public_abi.ResultCode, data: collections.abc.Buffer, /
	) -> None: ...
	@abc.abstractmethod
	def has_result(self) -> bool: ...

	@abc.abstractmethod
	async def get_leader_nondet_result(
		self, call_no: int, /
	) -> collections.abc.Buffer: ...
	@abc.abstractmethod
	async def post_nondet_result(
		self, call_no: int, data: collections.abc.Buffer, /
	) -> None: ...
	@abc.abstractmethod
	async def post_message(
		self, account: bytes, calldata: bytes, data: DefaultTransactionData, /
	) -> None: ...
	@abc.abstractmethod
	async def deploy_contract(
		self, calldata: bytes, code: bytes, data: DeployDefaultTransactionData, /
	) -> None: ...
	@abc.abstractmethod
	async def consume_gas(self, gas: int, /) -> None: ...
	@abc.abstractmethod
	async def eth_send(
		self, account: bytes, calldata: bytes, data: DefaultEthTransactionData, /
	) -> None: ...
	@abc.abstractmethod
	async def eth_call(self, account: bytes, calldata: bytes, /) -> bytes: ...
	@abc.abstractmethod
	async def get_balance(self, account: bytes, /) -> int: ...
	@abc.abstractmethod
	async def remaining_fuel_as_gen(self, /) -> int: ...
	@abc.abstractmethod
	async def post_event(self, topics: list[bytes], blob: bytes, /) -> None: ...
	@abc.abstractmethod
	async def notify_nondet_disagreement(self, call_no: int, /) -> None: ...


def save_code_callback[T](
	code: bytes, cb: typing.Callable[[bytes, int, bytes], T]
) -> tuple[T, T]:
	import hashlib

	code_digest = hashlib.sha3_256(b'\x00' * 32)
	CODE_OFFSET = 1
	code_digest.update(CODE_OFFSET.to_bytes(4, byteorder='little'))
	code_slot = code_digest.digest()
	r1 = cb(code_slot, 0, len(code).to_bytes(4, byteorder='little', signed=False))

	r2 = cb(code_slot, 4, code)

	return (r1, r2)


async def save_code_to_host(host: IHost, code: bytes):
	r1, r2 = save_code_callback(code, host.storage_write)
	await r1
	await r2


async def host_loop(handler: IHost, cancellation: asyncio.Event, *, logger: Logger):
	async_loop = asyncio.get_event_loop()

	logger.trace('entering loop')
	sock = await handler.loop_enter(cancellation)
	logger.trace('leaving loop')

	async def send_all(data: collections.abc.Buffer):
		await async_loop.sock_sendall(sock, data)

	async def read_exact(le: int) -> bytes:
		buf = bytearray([0] * le)
		idx = 0
		while idx < le:
			read = await async_loop.sock_recv_into(sock, memoryview(buf)[idx:le])
			if read == 0:
				raise ConnectionResetError()
			idx += read
		return bytes(buf)

	async def recv_int(bytes: int = 4) -> int:
		return int.from_bytes(await read_exact(bytes), byteorder='little', signed=False)

	async def send_int(i: int, bytes=4):
		await send_all(int.to_bytes(i, bytes, byteorder='little', signed=False))

	async def read_slice() -> memoryview:
		le = await recv_int()
		data = await read_exact(le)
		return memoryview(data)

	while True:
		meth_id = host_fns.Methods(await recv_int(1))
		logger.trace('got method', method=meth_id)
		match meth_id:
			case host_fns.Methods.GET_CALLDATA:
				try:
					cd = await handler.get_calldata()
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
					await send_int(len(cd))
					await send_all(cd)
			case host_fns.Methods.STORAGE_READ:
				mode = await read_exact(1)
				mode = public_abi.StorageType(mode[0])
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				slot = await read_exact(SLOT_ID_SIZE)
				index = await recv_int()
				le = await recv_int()
				try:
					res = await handler.storage_read(mode, account, slot, index, le)
					assert len(res) == le
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
					await send_all(res)
			case host_fns.Methods.STORAGE_WRITE:
				slot = await read_exact(SLOT_ID_SIZE)
				index = await recv_int()
				le = await recv_int()
				got = await read_exact(le)
				try:
					await handler.storage_write(slot, index, got)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
			case host_fns.Methods.CONSUME_RESULT:
				res = await read_slice()
				await handler.consume_result(public_abi.ResultCode(res[0]), res[1:])
				await send_all(b'\x00')
				return
			case host_fns.Methods.GET_LEADER_NONDET_RESULT:
				call_no = await recv_int()
				try:
					data = await handler.get_leader_nondet_result(call_no)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
					data = memoryview(data)
					await send_int(len(data))
					await send_all(data)
			case host_fns.Methods.POST_NONDET_RESULT:
				call_no = await recv_int()
				try:
					await handler.post_nondet_result(call_no, await read_slice())
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
			case host_fns.Methods.POST_MESSAGE:
				account = await read_exact(ACCOUNT_ADDR_SIZE)

				calldata_len = await recv_int()
				calldata = await read_exact(calldata_len)

				message_data_len = await recv_int()
				message_data_bytes = await read_exact(message_data_len)
				message_data = json.loads(str(message_data_bytes, 'utf-8'))

				try:
					await handler.post_message(account, calldata, message_data)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
			case host_fns.Methods.CONSUME_FUEL:
				gas = await recv_int(8)
				await handler.consume_gas(gas)
			case host_fns.Methods.DEPLOY_CONTRACT:
				calldata_len = await recv_int()
				calldata = await read_exact(calldata_len)

				code_len = await recv_int()
				code = await read_exact(code_len)

				message_data_len = await recv_int()
				message_data_bytes = await read_exact(message_data_len)
				message_data = json.loads(str(message_data_bytes, 'utf-8'))

				try:
					await handler.deploy_contract(calldata, code, message_data)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))

			case host_fns.Methods.ETH_SEND:
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				calldata_len = await recv_int()
				calldata = await read_exact(calldata_len)

				message_data_len = await recv_int()
				message_data_bytes = await read_exact(message_data_len)
				message_data = json.loads(str(message_data_bytes, 'utf-8'))

				try:
					await handler.eth_send(account, calldata, message_data)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
			case host_fns.Methods.ETH_CALL:
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				calldata_len = await recv_int()
				calldata = await read_exact(calldata_len)

				try:
					res = await handler.eth_call(account, calldata)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
					await send_int(len(res))
					await send_all(res)
			case host_fns.Methods.GET_BALANCE:
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				try:
					res = await handler.get_balance(account)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
					await send_all(res.to_bytes(32, byteorder='little', signed=False))
			case host_fns.Methods.REMAINING_FUEL_AS_GEN:
				try:
					res = await handler.remaining_fuel_as_gen()
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					res = min(res, 2**53 - 1)
					await send_all(bytes([host_fns.Errors.OK]))
					await send_all(res.to_bytes(8, byteorder='little', signed=False))
			case host_fns.Methods.POST_EVENT:
				topics_len = await recv_int(1)
				topics = []
				for i in range(topics_len):
					topic = await read_exact(32)
					topics.append(topic)
				blob = await read_slice()
				try:
					await handler.post_event(topics, blob)
				except HostException as e:
					await send_all(bytes([e.error_code]))
				else:
					await send_all(bytes([host_fns.Errors.OK]))
			case host_fns.Methods.NOTIFY_NONDET_DISAGREEMENT:
				call_no = await recv_int()
				await handler.notify_nondet_disagreement(call_no)
				# No response needed according to the spec
			case x:
				raise Exception(f'unknown method {x}')


@dataclass
class RunHostAndProgramRes:
	stdout: str
	stderr: str
	genvm_log: str


async def _send_timeout(manager_uri: str, genvm_id: str, logger: Logger):
	async with aiohttp.request(
		'DELETE',
		f'{manager_uri}/genvm/{genvm_id}?wait_timeout_ms=20',
	) as resp:
		logger.debug('delete /genvm', genvm_id=genvm_id, status=resp.status)
		if resp.status != 200:
			logger.debug('delete /genvm failed', genvm_id=genvm_id, body=await resp.text())


async def run_genvm(
	handler: IHost,
	*,
	timeout: float | None = None,
	manager_uri: str = 'http://127.0.0.1:3999',
	logger: Logger | None = None,
	is_sync: bool,
	capture_output: bool = True,
	message: typing.Any,
	host_data: str = '',
	host: str,
	extra_args: list[str] = [],
) -> RunHostAndProgramRes:
	if logger is None:
		logger = NoLogger()

	genvm_id_cell: list[str | None] = [None]
	status_cell: list[dict | Exception | None] = [None]
	cancellation_event = asyncio.Event()

	async def wrap_proc():
		try:
			max_exec_mins = 20
			if timeout is not None:
				max_exec_mins = int(max(max_exec_mins, (timeout * 1.5 + 59) // 60))

			timestamp = message.get('datetime', '2024-11-26T06:42:42.424242Z')

			async with aiohttp.request(
				'POST',
				f'{manager_uri}/genvm/run',
				json={
					'major': 0,  # FIXME
					'message': message,
					'is_sync': is_sync,
					'capture_output': capture_output,
					'host_data': host_data,
					'max_execution_minutes': max_exec_mins,  # this parameter is needed to prevent zombie genvms
					'timestamp': timestamp,
					'host': host,
					'extra_args': extra_args,
				},
			) as resp:
				logger.debug('post /genvm/run', status=resp.status)
				data = await resp.json()
				logger.trace('post /genvm/run', body=data)
				if resp.status != 200:
					logger.error(
						f'genvm manager /genvm/run failed', status=resp.status, body=data
					)
					raise Exception(f'genvm manager /genvm/run failed: {resp.status} {data}')
				else:
					genvm_id = data['id']
					logger.debug('genvm manager /genvm', genvm_id=genvm_id, status=resp.status)
					genvm_id_cell[0] = genvm_id
		finally:
			logger.debug('proc started', genvm_id=genvm_id_cell[0])

	async def wrap_host():
		await host_loop(handler, cancellation_event, logger=logger)
		logger.debug('host loop finished')

	async def wrap_timeout():
		if timeout is None:
			return
		await asyncio.sleep(timeout)
		genvm_id = genvm_id_cell[0]
		if genvm_id is None:
			return

		logger.warning('timeout reached, deleting genvm', genvm_id=genvm_id)
		await _send_timeout(manager_uri, genvm_id, logger)

	poll_status_mutex = asyncio.Lock()

	async def poll_status(genvm_id: str):
		async with poll_status_mutex:
			old_status = status_cell[0]
			if old_status is not None:
				return old_status
			async with aiohttp.request(
				'GET',
				f'{manager_uri}/genvm/{genvm_id}',
			) as resp:
				logger.debug('get /genvm', genvm_id=genvm_id, status=resp.status)
				body = await resp.json()
				logger.trace('get /genvm', genvm_id=genvm_id, body=body)
				if resp.status != 200 and body['status'] is not None:
					new_res = Exception(f'genvm manager /genvm failed: {resp.status} {body}')
				else:
					new_res = typing.cast(dict, body['status'])
			status_cell[0] = new_res
			return new_res

	async def prob_died():
		await asyncio.wait(
			[
				asyncio.ensure_future(asyncio.sleep(1)),
				asyncio.ensure_future(cancellation_event.wait()),
			],
			return_when=asyncio.FIRST_COMPLETED,
		)
		genvm_id = genvm_id_cell[0]
		if genvm_id is None:
			return
		status = await poll_status(genvm_id)
		if status is not None and not cancellation_event.is_set():
			logger.error('genvm died without connecting', genvm_id=genvm_id, status=status)
			cancellation_event.set()

	fut_host = asyncio.ensure_future(wrap_host())
	fut_proc = asyncio.ensure_future(wrap_proc())
	fut_timeout = asyncio.ensure_future(wrap_timeout())
	await asyncio.wait([fut_host, fut_proc, asyncio.ensure_future(prob_died())])
	fut_timeout.cancel()

	exceptions: list[Exception] = []
	try:
		fut_host.result()
	except Exception as e:
		exceptions.append(e)
	try:
		fut_proc.result()
	except Exception as e:
		exceptions.append(e)

	if len(exceptions) > 0:
		raise Exception(*exceptions) from exceptions[0]

	genvm_id = genvm_id_cell[0]
	if genvm_id is not None:
		await _send_timeout(manager_uri, genvm_id, logger)

		status = await poll_status(genvm_id)
		if isinstance(status, Exception):
			exceptions.append(status)
		if len(exceptions) > 0:
			final_exception = Exception('execution failed', exceptions[1:])
			raise final_exception from exceptions[0]
		return RunHostAndProgramRes(
			stdout=status['stdout'],
			stderr=status['stderr'],
			genvm_log='# currently absent',
		)

	raise Exception('Execution failed')


async def run_host_and_program(
	handler: IHost,
	program: list[Path | str],
	*,
	env=None,
	cwd: Path | None = None,
	exit_timeout=0.05,
	deadline: float | None = None,
) -> RunHostAndProgramRes:
	loop = asyncio.get_running_loop()

	async def connect_reader(fd):
		reader = asyncio.StreamReader(loop=loop)
		reader_proto = asyncio.StreamReaderProtocol(reader)
		transport, _ = await loop.connect_read_pipe(
			lambda: reader_proto, os.fdopen(fd, 'rb')
		)
		return reader, transport

	stdout_rfd, stdout_wfd = os.pipe()
	stderr_rfd, stderr_wfd = os.pipe()
	genvm_log_rfd, genvm_log_wfd = os.pipe()
	stdout_reader, stdout_transport = await connect_reader(stdout_rfd)
	stderr_reader, stderr_transport = await connect_reader(stderr_rfd)
	genvm_log_reader, genvm_log_transport = await connect_reader(genvm_log_rfd)

	run_idx = program.index('run')
	program.insert(run_idx, '--log-fd')
	program.insert(run_idx + 1, str(genvm_log_wfd))

	process = await asyncio.create_subprocess_exec(
		*program,
		stdin=asyncio.subprocess.DEVNULL,
		stdout=stdout_wfd,
		stderr=stderr_wfd,
		cwd=cwd,
		env=env,
		pass_fds=(genvm_log_wfd,),
	)
	os.close(stdout_wfd)
	os.close(stderr_wfd)
	os.close(genvm_log_wfd)
	if process.stdin is not None:
		process.stdin.close()

	async def read_whole(reader, transport, put_to: list[bytes]):
		try:
			while True:
				read = await reader.read(4096)
				if read is None or len(read) == 0:
					break
				put_to.append(read)

				# print(program, read)
		finally:
			try:
				transport.close()
			except OSError:
				pass
			await asyncio.sleep(0)

	stdout, stderr, genvm_log = [], [], []

	cancellation_event = asyncio.Event()

	async def wrap_proc():
		try:
			await asyncio.gather(
				read_whole(stdout_reader, stdout_transport, stdout),
				read_whole(stderr_reader, stderr_transport, stderr),
				read_whole(genvm_log_reader, genvm_log_transport, genvm_log),
				process.wait(),
			)
		finally:
			cancellation_event.set()

	coro_proc = asyncio.ensure_future(wrap_proc())

	async def wrap_host():
		await host_loop(handler, cancellation_event)

	coro_loop = asyncio.ensure_future(wrap_host())

	all_proc = [coro_loop, coro_proc]
	deadline_future: None | asyncio.Task[None] = None
	if deadline is not None:
		deadline_future = asyncio.ensure_future(asyncio.sleep(deadline))
		all_proc.append(deadline_future)

	done, _pending = await asyncio.wait(
		all_proc,
		return_when=asyncio.FIRST_COMPLETED,
	)

	errors = []

	for x in done:
		try:
			x.result()
		except ConnectionResetError:
			pass
		except Exception as e:
			errors.append(e)

	# coro_loop must finish first if everything succeeded
	if not coro_loop.done() and not handler.has_result() and deadline is None:
		print('WARNING: genvm finished first')
		coro_loop.cancel()

	async def wait_all_timeout():
		timeout = asyncio.ensure_future(asyncio.sleep(exit_timeout))
		all_futs = [timeout, coro_proc]
		if not coro_loop.done():
			all_futs.append(coro_loop)
		done, _pending = await asyncio.wait(
			all_futs,
			return_when=asyncio.FIRST_COMPLETED,
		)
		if coro_loop in done:
			await wait_all_timeout()

	if handler.has_result():
		await wait_all_timeout()

	if not coro_proc.done():
		try:
			process.terminate()
		except:
			pass
		await wait_all_timeout()
		if not coro_proc.done():
			# genvm exit takes to long, forcefully quit it
			try:
				process.kill()
			except:
				pass

	try:
		await coro_loop
	except ConnectionResetError:
		pass
	except (Exception, asyncio.CancelledError) as e:
		errors.append(e)

	exit_code = await process.wait()

	if not handler.has_result():
		if (
			deadline_future is None
			or deadline_future is not None
			and deadline_future not in done
		):
			errors.append(Exception('no result provided'))
		else:
			await handler.consume_result(public_abi.ResultCode.VM_ERROR, b'timeout')

	result = RunHostAndProgramRes(
		b''.join(stdout).decode(),
		b''.join(stderr).decode(),
		b''.join(genvm_log).decode(),
	)

	if len(errors) > 0:
		raise Exception(
			*errors,
			{
				'stdout': result.stdout,
				'stderr': result.stderr,
				'genvm_log': result.genvm_log,
			},
		) from errors[0]

	return result
