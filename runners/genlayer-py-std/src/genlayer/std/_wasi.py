import typing

if typing.TYPE_CHECKING:
	import collections.abc

	type _Fd = int

	def rollback(s: str) -> typing.NoReturn: ...

	def contract_return(s: bytes) -> typing.NoReturn: ...

	# returns fd
	def run_nondet(eq_principle: bytes, calldata: bytes) -> int: ...

	# returns fd
	def call_contract(address: bytes, calldata: bytes) -> int: ...

	def post_message(address: bytes, calldata: bytes, gas: int, code: bytes) -> int: ...

	def get_message_data() -> str: ...

	def get_entrypoint() -> bytes: ...

	def get_webpage(config: str, url: str) -> _Fd: ...

	def exec_prompt(config: str, prompt: str) -> _Fd: ...

	def exec_prompt_id(id: int, vars: str) -> _Fd: ...

	def eq_principle_prompt(id: int, vars: str) -> bool: ...

	def storage_read(slot: bytes, off: int, len: int) -> bytes: ...
	def storage_write(slot: bytes, off: int, what: collections.abc.Buffer) -> bytes: ...
else:
	import os

	if not os.getenv('GENERATING_DOCS', 'false') == 'true':
		from _genlayer_wasi import *
	else:

		def get_message_data() -> str:
			return """
			{
				"contract_account": "0x0000000000000000000000000000000000000000",
				"sender_account": "0x0000000000000000000000000000000000000000"
			}
			"""
