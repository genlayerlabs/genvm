from pathlib import Path
import sys
if __name__ == '__main__':
	script_dir = Path(__file__).parent.absolute()
	root_dir = script_dir
	while not root_dir.joinpath('.genvm-monorepo-root').exists():
		root_dir = root_dir.parent
	sys.path.append(str(root_dir.joinpath('sdk-python', 'py')))

import genlayer.types

import socket
import threading
import typing
from genlayer.types import Address
import pickle

def _handle_exc(e):
	if isinstance(e, (AbortThread, ConnectionResetError)):
		return
	import traceback
	traceback.print_exception(e)

class MockStorage:
	_storages: dict[Address, dict[Address, bytearray]]
	def __init__(self):
		self._storages = {}
	def read(self, gas_before: int, account: Address, slot: Address, index: int, le: int) -> tuple[bytes, int]:
		res = self._storages.setdefault(account, {})
		res = res.setdefault(slot, bytearray())
		return (res[index : index+le] + b"\x00" * (le - max(0, len(res) - index)), gas_before)
	def write(self, gas_before: int, account: Address, slot: Address, index: int, what: memoryview) -> int:
		res = self._storages.setdefault(account, {})
		res = res.setdefault(slot, bytearray())
		res.extend(b"\x00" * (index + len(what) - len(res)))
		memoryview(res)[index:index + len(what)] = what
		return gas_before

class AbortThread(Exception):
	pass

class MockHost:
	thread: threading.Thread

	def __init__(self, *, path: str, calldata: bytes, storage_path_pre: Path, storage_path_post: Path, codes: dict[bytes, typing.Any]):
		self.path = path
		self.calldata = calldata
		self.storage_path_pre = storage_path_pre
		self.storage_path_post = storage_path_post
		self.codes = codes
		self.storage = None
		self.sock = None
		self.thead = None
	def __enter__(self):
		self.created = False
		Path(self.path).unlink(missing_ok=True)
		self.thread_should_stop = False
		with open(self.storage_path_pre, 'rb') as f:
			self.storage = pickle.load(f)
		self.thread = threading.Thread(target=lambda: self._threadfn(), daemon=True)
		self.thread.start()
		return self
	def _threadfn(self):
		try:
			with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock_listener:
				sock_listener.settimeout(0.1)
				sock_listener.bind(self.path)
				sock_listener.listen(1)
				self.created = True
				not_accept = True
				while not_accept:
					try:
						sock, _ = sock_listener.accept()
						sock.settimeout(0.1)
						not_accept = False
					except socket.timeout:
						if self.thread_should_stop:
							raise AbortThread()
				buf = bytearray([0]* 4)
				def read_exact(le, idx0=0):
					buf.extend(b"\x00" * (idx0 + le - len(buf)))
					idx = idx0
					while idx < le:
						if self.thread_should_stop:
							raise AbortThread()
						try:
							idx += sock.recv_into(memoryview(buf)[idx:], idx0 + le - idx)
						except socket.timeout:
							pass
				def read_exact_get(le, idx0=0):
					read_exact(le, idx0)
					return bytes(buf[idx0:idx0+le])
				def recv_int(bytes=4) -> int:
					read_exact(bytes)
					return int.from_bytes(buf[:bytes], byteorder='little', signed=False)
				def send_int(i: int, bytes=4):
					sock.sendall(int.to_bytes(i, bytes, byteorder='little', signed=False))
				while not self.thread_should_stop:
					read_exact(1)
					match buf[0]:
						case 0: # get calldata
							send_int(len(self.calldata))
							sock.sendall(self.calldata)
						case 1: # get_code
							addr = Address(read_exact_get(32))
							res = self.codes.get(addr, None)
							if res is not None:
								res = res.get("code", None)
							if res is None:
								sock.sendall(b"\x01")
							else:
								with open(res, "rb") as f:
									contents = f.read()
								sock.sendall(b"\x00")
								send_int(len(contents))
								sock.sendall(contents)
						case 2: # storage_read
							gas_before = recv_int(8)
							account = Address(read_exact_get(32))
							slot = Address(read_exact_get(32))
							index = recv_int()
							le = recv_int()
							res, gas = self.storage.read(gas_before, account, slot, index, le)
							assert len(res) == le
							sock.sendall(b"\x00")
							send_int(gas, 8)
							sock.sendall(res)
						case 3: # storage write
							gas_before = recv_int(8)
							account = Address(read_exact_get(32))
							slot = Address(read_exact_get(32))
							index = recv_int()
							le = recv_int()
							read_exact(le)
							gas = self.storage.write(gas_before, account, slot, index, memoryview(buf)[:le])
							sock.sendall(b"\x00")
							send_int(gas, 8)
						case x:
							raise Exception(f"unknown method {x}")
		except Exception as e:
			self.thread_should_stop = True
			_handle_exc(e)
	def __exit__(self, *_args):
		if self.thread is not None:
			self.thread_should_stop = True
			self.thread.join()
			self.thread = None
		if self.storage is not None:
			with open(self.storage_path_post, 'wb') as f:
				pickle.dump(self.storage, f)
			self.storage = None
		Path(self.path).unlink(missing_ok=True)

if __name__ == '__main__':
	import time
	import base64
	with pickle.loads(base64.b64decode(sys.argv[1])) as host:
		while not host.thread_should_stop:
			time.sleep(0.2)
