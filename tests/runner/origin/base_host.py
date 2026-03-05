import socket
import typing
import collections.abc
import asyncio
import sys
import abc
import json
import time

import aiohttp

from dataclasses import dataclass

from pathlib import Path

if True:
	root = Path(__file__).parent
	while root.parent != root:
		mr_root = root.joinpath('.genvm-monorepo-root')
		if mr_root.exists():
			dir_to_add = json.loads(mr_root.read_bytes())['py-std']
			sys.path.insert(0, str(root.joinpath(*dir_to_add)))
			break
		root = root.parent
	import genlayer.py.calldata as gvm_calldata
	from genlayer.py.types import Address


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


class Message(typing.TypedDict):
	contract_address: Address
	sender_address: Address
	origin_address: Address
	chain_id: int
	value: typing.NotRequired[int]
	is_init: bool
	datetime: typing.NotRequired[str]


class FingerprintFrame(typing.TypedDict):
	module_name: str
	func: int


class ResultFingerprint(typing.TypedDict):
	frames: list[FingerprintFrame]
	module_instances: dict[str, typing.Any]


class EthSendInner(typing.TypedDict):
	type: typing.Literal['EthSend']
	address: Address
	calldata: bytes
	value: int


class PostMessageInner(typing.TypedDict):
	type: typing.Literal['PostMessage']
	address: Address
	calldata: gvm_calldata.Decoded
	value: int
	on: typing.Literal['finalized', 'accepted']


class DeployContractInner(typing.TypedDict):
	type: typing.Literal['DeployContract']
	calldata: gvm_calldata.Decoded
	code: bytes
	value: int
	on: typing.Literal['finalized', 'accepted']
	salt_nonce: int


class EmitEventInner(typing.TypedDict):
	type: typing.Literal['EmitEvent']
	topics: list[bytes]
	blob: dict[str, gvm_calldata.Decoded]


type ResultEmission = typing.Union[
	EthSendInner,
	PostMessageInner,
	DeployContractInner,
	EmitEventInner,
]


class IHost(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def loop_enter(self, cancellation: asyncio.Event) -> socket.socket: ...

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
	async def consume_gas(self, gas: int, /) -> None: ...
	@abc.abstractmethod
	async def eth_call(self, account: bytes, calldata: bytes, /) -> bytes: ...
	@abc.abstractmethod
	async def get_balance(self, account: bytes, /) -> int: ...
	@abc.abstractmethod
	async def remaining_fuel_as_gen(self, /) -> int: ...
	@abc.abstractmethod
	async def notify_nondet_disagreement(self, call_no: int, /) -> None: ...


async def host_loop(
	handler: IHost, cancellation: asyncio.Event, *, logger: Logger
) -> None:
	async_loop = asyncio.get_event_loop()

	logger.trace('entering loop')
	sock = await handler.loop_enter(cancellation)
	logger.trace('entered loop')

	socket_write_buffer = bytearray()
	socket_read_buffer = bytearray(b'\x00' * 4096)

	async def send_all(data: bytes | memoryview):
		socket_write_buffer.extend(data)
		if len(socket_write_buffer) > 4096:
			await flush_socket_buffer()

	async def flush_socket_buffer():
		if len(socket_write_buffer) > 0:
			await async_loop.sock_sendall(sock, socket_write_buffer)
			socket_write_buffer.clear()

	socket_read_buf = bytearray(65536)
	socket_read_buf_view = memoryview(socket_read_buf)
	socket_read_start = 0
	socket_read_end = 0

	async def read_exact(le: int) -> bytes:
		nonlocal socket_read_start, socket_read_end
		out = bytearray(le)
		idx = 0
		while idx < le:
			available = socket_read_end - socket_read_start
			if available == 0:
				socket_read_start = 0
				socket_read_end = await async_loop.sock_recv_into(sock, socket_read_buf_view)
				if socket_read_end == 0:
					raise ConnectionResetError()
				available = socket_read_end
			take = min(available, le - idx)
			out[idx : idx + take] = socket_read_buf[
				socket_read_start : socket_read_start + take
			]
			idx += take
			socket_read_start += take
		return bytes(out)

	async def recv_int(bytes: int = 4) -> int:
		return int.from_bytes(await read_exact(bytes), byteorder='little', signed=False)

	async def send_int(i: int, bytes=4):
		await send_all(int.to_bytes(i, bytes, byteorder='little', signed=False))

	async def read_slice() -> memoryview:
		le = await recv_int()
		data = await read_exact(le)
		return memoryview(data)

	total_handling_time = 0.0
	time_per_method = {}
	call_counts = {}
	meth_id: host_fns.Methods | None = None

	handling_start = time.time()
	while True:
		cur_delta = time.time() - handling_start
		if meth_id is not None:
			total_handling_time += cur_delta
			time_per_method[meth_id.name] = time_per_method.get(meth_id.name, 0.0) + cur_delta

		await flush_socket_buffer()

		meth_id = host_fns.Methods(await recv_int(1))
		logger.trace('got method', method=meth_id, method_name=meth_id.name)
		call_counts[meth_id.name] = call_counts.get(meth_id.name, 0) + 1

		handling_start = time.time()
		match meth_id:
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
			case host_fns.Methods.CONSUME_RESULT:
				raise Exception(
					'CONSUME_RESULT is not supported in this host loop implementation, use manager provided one'
				)
			case host_fns.Methods.CONSUME_FUEL:
				gas = await recv_int(8)
				await handler.consume_gas(gas)
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
			case host_fns.Methods.NOTIFY_NONDET_DISAGREEMENT:
				call_no = await recv_int()
				await handler.notify_nondet_disagreement(call_no)
				# No response needed according to the spec
			case host_fns.Methods.NOTIFY_FINISHED:
				logger.debug(
					'handling time',
					total=total_handling_time,
					by_method=time_per_method,
					call_counts=call_counts,
				)
				await send_all(bytes([0]))
				await flush_socket_buffer()
				return None
			case x:
				raise Exception(f'unknown method {x}')


@dataclass
class RunHostAndProgramRes:
	stdout: str
	stderr: str
	genvm_log: list[dict[str, typing.Any]]

	execution_time: float

	execution_hash: bytes

	result_kind: public_abi.ResultCode
	result_data: gvm_calldata.Decoded
	result_fingerprint: ResultFingerprint | None
	result_storage_changes: list[tuple[bytes, bytes]]
	result_emissions: list[ResultEmission]
	result_nondet_results: list[bytes]
	vm_error_description: str | None = None


async def _send_timeout(manager_uri: str, genvm_id: str, logger: Logger):
	try:
		async with aiohttp.request(
			'DELETE',
			f'{manager_uri}/genvm/{genvm_id}?wait_timeout_ms=20',
		) as resp:
			logger.debug('delete /genvm', genvm_id=genvm_id, status=resp.status)
			if resp.status != 200:
				logger.warning(
					'delete /genvm failed', genvm_id=genvm_id, body=await resp.text()
				)
	except (aiohttp.ClientError, asyncio.TimeoutError) as exc:
		logger.warning('delete /genvm request failed', genvm_id=genvm_id, error=str(exc))


async def run_genvm(
	handler: IHost,
	*,
	timeout: float | None = None,
	manager_uri: str = 'http://127.0.0.1:3999',
	logger: Logger | None = None,
	is_sync: bool,
	capture_output: bool = True,
	message: Message,
	host_data: str = '',
	host: str,
	extra_args: list[str] = [],
	storage_pages: int = 10_000_000,
	receipt_words: int = 10_000_000,
	code: bytes | None = None,
	calldata: bytes,
	leader_nondet_results: list[bytes] | None = None,
	request_extra: dict[str, gvm_calldata.Encodable] = {},
) -> RunHostAndProgramRes:
	if logger is None:
		logger = NoLogger()

	genvm_id_cell: list[str | None] = [None]
	status_cell: list[dict | Exception | None] = [None]
	cancellation_event = asyncio.Event()

	started_at = [time.time()]

	async def wrap_proc():
		try:
			max_exec_mins = 20
			if timeout is not None:
				max_exec_mins = int(max(max_exec_mins, (timeout * 1.5 + 59) // 60))

			timestamp = message.get('datetime', '2024-11-26T06:42:42.424242Z')

			async with aiohttp.request(
				'POST',
				f'{manager_uri}/genvm/run',
				data=gvm_calldata.encode(
					{
						'major': 0,  # FIXME
						'message': message,
						'is_sync': is_sync,
						'capture_output': capture_output,
						'host_data': host_data,
						'max_execution_minutes': max_exec_mins,  # this parameter is needed to prevent zombie genvms
						'timestamp': timestamp,
						'host': host,
						'extra_args': extra_args,
						'storage_pages': storage_pages,
						'receipt_words': receipt_words,
						'code': code,
						'calldata': calldata,
						'leader_nondet_results': leader_nondet_results,
						**request_extra,
					}
				),
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
					asyncio.ensure_future(wrap_timeout(genvm_id))
		finally:
			logger.debug('proc started', genvm_id=genvm_id_cell[0])
			started_at[0] = time.time()

	async def wrap_host():
		await host_loop(handler, cancellation_event, logger=logger)
		logger.debug('host loop finished')

	timeout_fired = asyncio.Event()

	async def wrap_timeout(genvm_id: str):
		if timeout is None:
			return
		await asyncio.sleep(timeout)
		logger.debug('timeout reached', genvm_id=genvm_id)
		timeout_fired.set()
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
				if resp.status != 200:
					new_res = Exception(f'genvm manager /genvm failed: {resp.status} {body}')
				elif body['status'] is None:
					return None
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
	await asyncio.wait([fut_host, fut_proc, asyncio.ensure_future(prob_died())])

	exceptions: list[Exception] = []
	result_host: tuple[public_abi.ResultCode, bytes] | None = None
	try:
		fut_host.result()
	except ConnectionResetError as e:
		if not timeout_fired.is_set():
			logger.warning('connection reset without timeout', error=e)
	except Exception as e:
		if not timeout_fired.is_set():
			exceptions.append(e)
		else:
			logger.warning('host handler failed after timeout', error=e)
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
		if status is None:
			exceptions.append(Exception('execution failed: no status'))
		elif isinstance(status, Exception):
			exceptions.append(status)
		if len(exceptions) > 0:
			final_exception = Exception('execution failed', exceptions[1:])
			raise final_exception from exceptions[0]

		# Result was sent to manager via consume_result, get it from status
		consumed_result_raw = (
			status.get('consumed_result') if isinstance(status, dict) else None
		)
		if consumed_result_raw is not None:
			consumed_result_bytes = bytes(consumed_result_raw)
			result_kind = public_abi.ResultCode(consumed_result_bytes[0])
			decoded = gvm_calldata.decode(consumed_result_bytes[1:])
			execution_hash = decoded.get('execution_hash', b'')
			result_data = decoded.get('data')
			result_fingerprint = decoded.get('fingerprint')
			result_storage_changes = decoded.get('storage_changes', [])
			result_emissions = decoded.get('emissions', [])
			nondet_results = decoded.get('nondet_results', [])
		else:
			execution_hash = b''
			result_kind = public_abi.ResultCode.INTERNAL_ERROR
			result_data = 'no_result'
			result_fingerprint = None
			result_storage_changes = []
			result_emissions = []
			nondet_results = []

		vm_error_description: str | None = None
		if result_kind == public_abi.ResultCode.VM_ERROR and isinstance(result_data, str):
			try:
				async with aiohttp.request(
					'GET',
					f'{manager_uri}/vm-error/describe',
					params={'error': result_data},
				) as resp:
					if resp.status == 200:
						body = await resp.json()
						vm_error_description = body.get('description')
			except Exception as e:
				logger.warning('failed to get vm error description', error=str(e))

		return RunHostAndProgramRes(
			stdout=status['stdout'],
			stderr=status['stderr'],
			genvm_log=status.get('genvm_log') or [],
			execution_hash=execution_hash,
			result_kind=result_kind,
			result_data=result_data,
			result_fingerprint=result_fingerprint,
			result_storage_changes=result_storage_changes,
			result_emissions=result_emissions,
			result_nondet_results=nondet_results,
			vm_error_description=vm_error_description,
			execution_time=time.time() - started_at[0],
		)

	raise Exception('Execution failed')
