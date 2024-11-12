from pathlib import Path
import sys

if __name__ == '__main__':
	import json

	MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
	script_dir = Path(__file__).parent.absolute()

	root_dir = script_dir
	while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
		root_dir = root_dir.parent
	MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())

	sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF['py-std'])))

from genlayer.py.types import Address
from genlayer.py import calldata as _calldata

import socket
import typing
import pickle
import io

from base_host import *


class MockStorage:
	_storages: dict[Address, dict[bytes, bytearray]]

	def __init__(self):
		self._storages = {}

	def read(
		self, gas_before: int, account: Address, slot: bytes, index: int, le: int
	) -> tuple[bytes, int]:
		res = self._storages.setdefault(account, {})
		res = res.setdefault(slot, bytearray())
		return (
			res[index : index + le] + b'\x00' * (le - max(0, len(res) - index)),
			gas_before,
		)

	def write(
		self,
		gas_before: int,
		account: Address,
		slot: bytes,
		index: int,
		what: collections.abc.Buffer,
	) -> int:
		res = self._storages.setdefault(account, {})
		res = res.setdefault(slot, bytearray())
		what = memoryview(what)
		res.extend(b'\x00' * (index + len(what) - len(res)))
		memoryview(res)[index : index + len(what)] = what
		return gas_before


class MockHost(IHost):
	sock: socket.socket | None
	storage: MockStorage | None
	messages_file: io.TextIOWrapper | None

	def __init__(
		self,
		*,
		path: str,
		calldata: bytes,
		messages_path: Path,
		storage_path_pre: Path,
		storage_path_post: Path,
		codes: dict[Address, typing.Any],
		leader_nondet,
	):
		self.path = path
		self.calldata = calldata
		self.storage_path_pre = storage_path_pre
		self.storage_path_post = storage_path_post
		self.leader_nondet = leader_nondet
		self.codes = codes
		self.storage = None
		self.sock = None
		self.thread = None
		self.messages_file = None
		self.messages_path = messages_path

	def __enter__(self):
		self.created = False
		Path(self.path).unlink(missing_ok=True)
		self.thread_should_stop = False
		with open(self.storage_path_pre, 'rb') as f:
			self.storage = pickle.load(f)

		self.sock_listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		self.sock_listener.bind(self.path)
		self.sock_listener.listen(1)

		return self

	def __exit__(self, *_args):
		if self.storage is not None:
			with open(self.storage_path_post, 'wb') as f:
				pickle.dump(self.storage, f)
			self.storage = None
		if self.messages_file is not None:
			self.messages_file.close()
			self.messages_file = None
		if self.sock is not None:
			self.sock.close()
		Path(self.path).unlink(missing_ok=True)

	async def loop_enter(self):
		async_loop = asyncio.get_event_loop()
		self.sock, _addr = await async_loop.sock_accept(self.sock_listener)
		self.sock.setblocking(False)
		self.sock_listener.close()
		self.sock_listener = None
		return self.sock

	async def get_calldata(self) -> bytes:
		return self.calldata

	async def get_code(self, addr_b: bytes) -> bytes:
		addr = Address(addr_b)
		res = self.codes.get(addr, None)
		if res is not None:
			res = res.get('code', None)
		if res is None:
			raise Exception(f'no code for {addr}')
		with open(res, 'rb') as f:
			return f.read()

	async def storage_read(
		self, gas_before: int, account: bytes, slot: bytes, index: int, le: int
	) -> tuple[bytes, int]:
		assert self.storage is not None
		return self.storage.read(gas_before, Address(account), slot, index, le)

	async def storage_write(
		self,
		gas_before: int,
		account: bytes,
		slot: bytes,
		index: int,
		got: collections.abc.Buffer,
	) -> int:
		assert self.storage is not None
		return self.storage.write(gas_before, Address(account), slot, index, got)

	async def consume_result(
		self, type: ResultCode, data: collections.abc.Buffer
	) -> None:
		pass

	async def get_leader_nondet_result(self, call_no: int) -> bytes | str | None:
		if self.leader_nondet is None:
			return None
		res = self.leader_nondet[call_no]
		if res['ok']:
			return _calldata.encode(res['value'])
		else:
			val = res['value']
			assert isinstance(val, str)
			return val

	async def post_nondet_result(
		self, call_no: int, type: ResultCode, data: collections.abc.Buffer
	):
		pass

	async def post_message(
		self, gas: int, account: bytes, calldata: bytes, code: bytes
	) -> None:
		if self.messages_file is None:
			self.messages_file = open(self.messages_path, 'wt')
			self.messages_file.write(f'{gas}\n{calldata}\n{code}\n')


if __name__ == '__main__':
	with pickle.loads(Path(sys.argv[1]).read_bytes()) as host:
		asyncio.run(host_loop(host))
