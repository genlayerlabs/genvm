# This file is auto-generated. Do not edit!

from enum import IntEnum, StrEnum
import typing


class StrTrieNode(typing.NamedTuple):
	head: str
	is_terminal: bool
	param: typing.Optional[str]
	children: tuple


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


class SpecialMethod(StrEnum):
	GET_SCHEMA = '#get-schema'
	ERRORED_MESSAGE = '#error'


VM_ERROR: typing.Final[tuple[StrTrieNode, ...]] = (
	StrTrieNode(head='timeout', is_terminal=True, param=None, children=()),
	StrTrieNode(head='exit_code', is_terminal=False, param='i32', children=()),
	StrTrieNode(head='wasm_trap', is_terminal=False, param='str', children=()),
	StrTrieNode(
		head='OOM',
		is_terminal=False,
		param=None,
		children=(
			StrTrieNode(
				head='RAM',
				is_terminal=True,
				param=None,
				children=(
					StrTrieNode(head='table', is_terminal=True, param=None, children=()),
					StrTrieNode(head='memory', is_terminal=True, param=None, children=()),
				),
			),
			StrTrieNode(head='Storage', is_terminal=True, param=None, children=()),
		),
	),
	StrTrieNode(
		head='invalid_contract',
		is_terminal=True,
		param=None,
		children=(
			StrTrieNode(
				head='absent_runner_comment', is_terminal=True, param=None, children=()
			),
			StrTrieNode(head='not_utf8_text', is_terminal=True, param=None, children=()),
			StrTrieNode(head='malformed_runner', is_terminal=True, param=None, children=()),
			StrTrieNode(
				head='wasm',
				is_terminal=False,
				param=None,
				children=(
					StrTrieNode(head='validating', is_terminal=True, param=None, children=()),
					StrTrieNode(head='linking', is_terminal=True, param=None, children=()),
					StrTrieNode(head='entrypoint', is_terminal=True, param=None, children=()),
				),
			),
		),
	),
	StrTrieNode(head='host', is_terminal=False, param='str', children=()),
)


EVENT_MAX_TOPICS: typing.Final[int] = 4


ABSENT_VERSION: typing.Final[str] = 'v0.1.0'


CODE_SLOT_OFFSET: typing.Final[int] = 1
