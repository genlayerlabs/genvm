from enum import IntEnum


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
