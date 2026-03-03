#!/usr/bin/env python3

"""
Compiles a Python source file to .pyc by deploying a compiler contract through the genvm manager.
This ensures deterministic bytecode (produced by the WASM CPython inside genvm).

Usage:
	python compile-py.py --src contract.py --out contract.pyc [--manager-url http://127.0.0.1:3999]
"""

import argparse
import asyncio
import json
import socket
import sys
import tempfile
from pathlib import Path

# Set up imports from monorepo
_root = Path(__file__).parent
while _root.parent != _root:
	mr_root = _root.joinpath('.genvm-monorepo-root')
	if mr_root.exists():
		_conf = json.loads(mr_root.read_bytes())
		sys.path.insert(0, str(_root.joinpath(*_conf['py-std'])))
		sys.path.insert(0, str(_root / 'tests' / 'runner'))
		break
	_root = _root.parent

import genlayer.py.calldata as gvm_calldata
from genlayer.py.types import Address
import origin.base_host as base_host
import origin.public_abi as public_abi
from origin.base_host import IHost, HostException
from origin import host_fns

COMPILER_CONTRACT = """\
# { "Depends": "py-genlayer:test" }

from genlayer import *
import importlib._bootstrap_external
import importlib.util

class Contract(gl.Contract):
	def __init__(self, name: str, code: bytes):
		opt_level = 0
		pyc_name = importlib.util.cache_from_source(name, optimization='')
		code_obj = compile(code, name, 'exec', dont_inherit=True, optimize=opt_level)
		source_hash = importlib.util.source_hash(code)
		bytecode = importlib._bootstrap_external._code_to_hash_pyc(
			code_obj,
			source_hash,
			False,
		)
		return bytecode
"""


class CompileHost(IHost):
	def __init__(self, sock_path: str):
		self.sock_path = sock_path
		self._sock_listener: socket.socket | None = None
		self._sock: socket.socket | None = None

	def __enter__(self):
		Path(self.sock_path).unlink(missing_ok=True)
		self._sock_listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		self._sock_listener.bind(self.sock_path)
		self._sock_listener.setblocking(False)
		self._sock_listener.listen(1)
		return self

	def __exit__(self, *_args):
		if self._sock is not None:
			self._sock.close()
		if self._sock_listener is not None:
			self._sock_listener.close()
		Path(self.sock_path).unlink(missing_ok=True)

	async def loop_enter(self, cancellation: asyncio.Event) -> socket.socket:
		loop = asyncio.get_event_loop()
		assert self._sock_listener is not None
		accept_fut = asyncio.ensure_future(loop.sock_accept(self._sock_listener))
		cancel_fut = asyncio.ensure_future(cancellation.wait())
		done, _pending = await asyncio.wait(
			[cancel_fut, accept_fut], return_when=asyncio.FIRST_COMPLETED
		)
		if cancel_fut in done:
			raise Exception('cancelled')
		cancellation.set()
		cancel_fut.cancel()
		self._sock, _addr = accept_fut.result()
		self._sock.setblocking(False)
		self._sock_listener.close()
		self._sock_listener = None
		return self._sock

	async def storage_read(self, mode, account, slot, index, le):
		return b'\x00' * le

	async def post_message(self, account, calldata, data):
		pass

	async def deploy_contract(self, calldata, code, data):
		pass

	async def consume_gas(self, gas):
		pass

	async def eth_send(self, account, calldata, data):
		pass

	async def eth_call(self, account, calldata):
		return b''

	async def get_balance(self, account):
		return 0

	async def remaining_fuel_as_gen(self):
		return 2**32

	async def notify_nondet_disagreement(self, call_no):
		pass


async def compile_py(src: Path, out: Path, manager_url: str) -> None:
	py_src = src.read_bytes()

	calldata = gvm_calldata.encode({'args': [src.name, py_src]})

	with tempfile.TemporaryDirectory(prefix='genvm-compile-') as tmp:
		sock_path = str(Path(tmp) / 'host.sock')
		host = CompileHost(sock_path)

		with host:
			res = await base_host.run_genvm(
				host,
				manager_uri=manager_url,
				message={
					'contract_address': Address(b'\x01' + b'\x00' * 19),
					'sender_address': Address(b'\x02' + b'\x00' * 19),
					'origin_address': Address(b'\x02' + b'\x00' * 19),
					'chain_id': 0,
					'value': 0,
					'is_init': True,
				},
				is_sync=False,
				host_data='{"node_address":"0x","tx_id":"0x"}',
				host='unix://' + sock_path,
				code=COMPILER_CONTRACT.encode('utf-8'),
				calldata=calldata,
				extra_args=['--debug-mode'],
			)

	if res.result_kind != public_abi.ResultCode.RETURN:
		print(f'compilation failed: {res.result_kind.name}', file=sys.stderr)
		print(f'result_data: {res.result_data}', file=sys.stderr)
		if res.stderr:
			print(f'stderr:\n{res.stderr}', file=sys.stderr)
		sys.exit(1)

	if not isinstance(res.result_data, bytes):
		print(f'unexpected result type: {type(res.result_data).__name__}', file=sys.stderr)
		sys.exit(1)

	out.parent.mkdir(parents=True, exist_ok=True)
	out.write_bytes(res.result_data)


def main():
	parser = argparse.ArgumentParser(
		description='Compile Python source to .pyc via genvm'
	)
	parser.add_argument('--src', required=True, type=Path, help='Input .py file')
	parser.add_argument('--out', required=True, type=Path, help='Output .pyc file')
	parser.add_argument(
		'--manager-url',
		default='http://127.0.0.1:3999',
		help='Manager URL (default: http://127.0.0.1:3999)',
	)
	args = parser.parse_args()

	if not args.src.exists():
		print(f'source file not found: {args.src}', file=sys.stderr)
		sys.exit(1)

	asyncio.run(compile_py(args.src, args.out, args.manager_url))


if __name__ == '__main__':
	main()
