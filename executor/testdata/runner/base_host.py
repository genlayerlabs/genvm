import socket
import typing
import collections.abc
import asyncio
import os

from dataclasses import dataclass

from pathlib import Path

if typing.TYPE_CHECKING:
	from .host_fns import *
else:
	from pathlib import Path

	exec(Path(__file__).parent.joinpath('host_fns.py').read_text())

ACCOUNT_ADDR_SIZE = 20
GENERIC_ADDR_SIZE = 32


class IHost(typing.Protocol):
	async def loop_enter(self) -> socket.socket: ...

	async def get_calldata(self, /) -> bytes: ...
	async def get_code(self, addr: bytes, /) -> bytes: ...
	async def storage_read(
		self, gas_before: int, account: bytes, slot: bytes, index: int, le: int, /
	) -> tuple[bytes, int]: ...
	async def storage_write(
		self,
		gas_before: int,
		account: bytes,
		slot: bytes,
		index: int,
		got: collections.abc.Buffer,
		/,
	) -> int: ...
	async def consume_result(
		self, type: ResultCode, data: collections.abc.Buffer, /
	) -> None: ...
	async def get_leader_nondet_result(self, call_no: int, /) -> bytes | str | None: ...
	async def post_nondet_result(
		self, call_no: int, type: ResultCode, data: collections.abc.Buffer, /
	) -> None: ...
	async def post_message(
		self, gas: int, account: bytes, calldata: bytes, code: bytes, /
	) -> None: ...


async def host_loop(handler: IHost):
	async_loop = asyncio.get_event_loop()

	sock = await handler.loop_enter()

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

	async def read_result() -> tuple[ResultCode, bytes]:
		type = await recv_int(1)
		le = await recv_int()
		data = await read_exact(le)
		return (ResultCode(type), data)

	while True:
		meth_id = Methods(await recv_int(1))
		match meth_id:
			case Methods.APPEND_CALLDATA:
				cd = await handler.get_calldata()
				await send_int(len(cd))
				await send_all(cd)
			case Methods.GET_CODE:
				addr = await read_exact(ACCOUNT_ADDR_SIZE)
				code = await handler.get_code(addr)
				await send_int(len(code))
				await send_all(code)
			case Methods.STORAGE_READ:
				gas_before = await recv_int(8)
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				slot = await read_exact(GENERIC_ADDR_SIZE)
				index = await recv_int()
				le = await recv_int()
				res, gas = await handler.storage_read(gas_before, account, slot, index, le)
				assert len(res) == le
				await send_int(gas, 8)
				await send_all(res)
			case Methods.STORAGE_WRITE:
				gas_before = await recv_int(8)
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				slot = await read_exact(GENERIC_ADDR_SIZE)
				index = await recv_int()
				le = await recv_int()
				got = await read_exact(le)
				gas = await handler.storage_write(gas_before, account, slot, index, got)
				await send_int(gas, 8)
			case Methods.CONSUME_RESULT:
				await handler.consume_result(*await read_result())
				await send_all(b'\x00')
				return
			case Methods.GET_LEADER_NONDET_RESULT:
				call_no = await recv_int()  # call no
				data = await handler.get_leader_nondet_result(call_no)
				if data is None:
					await send_all(bytes([ResultCode.NONE]))
				elif isinstance(data, str):
					await send_all(bytes([ResultCode.ROLLBACK]))
					encoded = data.encode('utf-8')
					await send_int(len(encoded))
					await send_all(encoded)
				else:
					await send_all(bytes([ResultCode.RETURN]))
					await send_int(len(data))
					await send_all(data)
			case Methods.POST_NONDET_RESULT:
				call_no = await recv_int()
				await handler.post_nondet_result(call_no, *await read_result())
			case Methods.POST_MESSAGE:
				account = await read_exact(ACCOUNT_ADDR_SIZE)
				gas = await recv_int(8)
				calldata_len = await recv_int()
				calldata = await read_exact(calldata_len)
				code_len = await recv_int()
				code = await read_exact(code_len)
				await handler.post_message(gas, account, calldata, code)
			case x:
				raise Exception(f'unknown method {x}')


import subprocess


@dataclass
class RunHostAndProgramRes:
	stdout: str
	stderr: str
	exceptions: list[Exception]


from concurrent.futures import ProcessPoolExecutor


async def run_host_and_program(
	handler: IHost,
	program: list[Path | str],
	*,
	env=None,
	cwd: Path | None = None,
	exit_timeout=0.05,
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
	stdout_reader, stdout_transport = await connect_reader(stdout_rfd)
	stderr_reader, stderr_transport = await connect_reader(stderr_rfd)

	process = await asyncio.create_subprocess_exec(
		program[0],
		*program[1:],
		stdin=asyncio.subprocess.DEVNULL,
		stdout=stdout_wfd,
		stderr=stderr_wfd,
		cwd=cwd,
		env=env,
	)
	os.close(stdout_wfd)
	os.close(stderr_wfd)
	if process.stdin is not None:
		process.stdin.close()

	async def read_whole(reader, transport, put_to: list[bytes]):
		try:
			while True:
				read = await reader.read(4096)
				if read is None or len(read) == 0:
					break
				put_to.append(read)
		finally:
			try:
				transport.close()
			except OSError:
				pass
			await asyncio.sleep(0)

	async def wrap_host():
		await host_loop(handler)

	stdout, stderr = [], []

	async def wrap_proc():
		await asyncio.gather(
			read_whole(stdout_reader, stdout_transport, stdout),
			read_whole(stderr_reader, stderr_transport, stderr),
			process.wait(),
		)

	coro_loop = asyncio.ensure_future(wrap_host())
	coro_proc = asyncio.ensure_future(wrap_proc())

	done, _pending = await asyncio.wait(
		[coro_loop, coro_proc],
		return_when=asyncio.FIRST_COMPLETED,
	)

	errors = []

	for x in done:
		try:
			x.result()
		except Exception as e:
			errors.append(e)

	# coro_loop must finish first if everything succeeded
	if not coro_loop.done():
		print('WARNING: genvm finished first')
		coro_loop.cancel()

	exit_code_use = True

	if not coro_proc.done():
		# genvm is exiting, let it clean all the resources for a bit
		await asyncio.wait(
			[coro_proc, asyncio.ensure_future(asyncio.sleep(exit_timeout))],
			return_when=asyncio.FIRST_COMPLETED,
		)
		if not coro_proc.done():
			# genvm exit takes to long, maybe it hanged. Politely ask to quit and wait a bit
			try:
				process.terminate()
			except:
				pass
			exit_code_use = False
			await asyncio.wait(
				[coro_proc, asyncio.ensure_future(asyncio.sleep(exit_timeout))],
				return_when=asyncio.FIRST_COMPLETED,
			)
			if not coro_proc.done():
				# genvm exit takes to long, forcefully quit it
				try:
					process.kill()
				except:
					pass

	await coro_proc
	exit_code = await process.wait()

	if exit_code_use and exit_code != 0:
		errors.append(Exception(f'exit code {exit_code} != 0'))

	return RunHostAndProgramRes(
		b''.join(stdout).decode(), b''.join(stderr).decode(), errors
	)
