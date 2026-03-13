# This file is auto-generated. Do not edit!

from enum import IntEnum, StrEnum
import typing


class ResultCode(IntEnum):
	RETURN = 0
	USER_ERROR = 1
	VM_ERROR = 2
	INTERNAL_ERROR = 3


class StorageType(IntEnum):
	DEFAULT = 0
	LATEST_FINAL = 1
	LATEST_NON_FINAL = 2


class EntryKind(IntEnum):
	MAIN = 0
	SANDBOX = 1
	CONSENSUS_STAGE = 2


class _MemoryLimiterConsts(typing.NamedTuple):
	TABLE_ENTRY: int = 64
	FILE_MAPPING: int = 256
	FD_ALLOCATION: int = 96


memory_limiter_consts: typing.Final = _MemoryLimiterConsts()


class _TopLimits(typing.NamedTuple):
	NONDET_BLOCKS: int = 4096
	LOCKED_SLOTS: int = 256
	UPGRADERS: int = 32
	VM_RECURSION: int = 512
	WEB_REQUEST_MIN_SPACE: int = 65536
	WEB_RENDER_MIN_SPACE: int = 134217728
	MAX_FDS: int = 1024


top_limits: typing.Final = _TopLimits()


class SpecialMethod(StrEnum):
	GET_SCHEMA = '#get-schema'
	ERRORED_MESSAGE = '#error'


class _VmErrorOomRam:
	@staticmethod
	def val() -> 'VmError':
		return VmError('OOM RAM')

	@staticmethod
	def table() -> 'VmError':
		return VmError('OOM RAM table')

	@staticmethod
	def memory() -> 'VmError':
		return VmError('OOM RAM memory')

	@staticmethod
	def limit() -> 'VmError':
		return VmError('OOM RAM limit')


class _VmErrorOom:
	@staticmethod
	def storage() -> 'VmError':
		return VmError('OOM storage')

	@staticmethod
	def receipt() -> 'VmError':
		return VmError('OOM receipt')

	@staticmethod
	def ram() -> '_VmErrorOomRam':
		return _VmErrorOomRam()


class _VmErrorInvalidContractWasm:
	@staticmethod
	def validating() -> 'VmError':
		return VmError('invalid_contract wasm validating')

	@staticmethod
	def linking() -> 'VmError':
		return VmError('invalid_contract wasm linking')

	@staticmethod
	def entrypoint() -> 'VmError':
		return VmError('invalid_contract wasm entrypoint')


class _VmErrorInvalidContract:
	@staticmethod
	def val() -> 'VmError':
		return VmError('invalid_contract')

	@staticmethod
	def absent_runner_comment() -> 'VmError':
		return VmError('invalid_contract absent_runner_comment')

	@staticmethod
	def not_utf8_text() -> 'VmError':
		return VmError('invalid_contract not_utf8_text')

	@staticmethod
	def malformed_runner() -> 'VmError':
		return VmError('invalid_contract malformed_runner')

	@staticmethod
	def wasm() -> '_VmErrorInvalidContractWasm':
		return _VmErrorInvalidContractWasm()


class _VmErrorExitCode:
	@staticmethod
	def val_i32(v: int) -> 'VmError':
		return VmError(f'exit_code {v}')


class _VmErrorWasmTrap:
	@staticmethod
	def val_str(v: str) -> 'VmError':
		return VmError(f'wasm_trap {v}')


class _VmErrorHost:
	@staticmethod
	def val_str(v: str) -> 'VmError':
		return VmError(f'host {v}')


class VmError:
	__slots__ = ('value',)

	def __init__(self, value: str):
		self.value = value

	def __str__(self) -> str:
		return self.value

	@staticmethod
	def timeout() -> 'VmError':
		return VmError('timeout')

	@staticmethod
	def absent_leader_nondet_output() -> 'VmError':
		return VmError('absent_leader_nondet_output')

	@staticmethod
	def exit_code() -> '_VmErrorExitCode':
		return _VmErrorExitCode()

	@staticmethod
	def wasm_trap() -> '_VmErrorWasmTrap':
		return _VmErrorWasmTrap()

	@staticmethod
	def oom() -> '_VmErrorOom':
		return _VmErrorOom()

	@staticmethod
	def invalid_contract() -> '_VmErrorInvalidContract':
		return _VmErrorInvalidContract()

	@staticmethod
	def host() -> '_VmErrorHost':
		return _VmErrorHost()


EVENT_MAX_TOPICS: typing.Final[int] = 4


ABSENT_VERSION: typing.Final[str] = 'v0.1.0'


CODE_SLOT_OFFSET: typing.Final[int] = 1
