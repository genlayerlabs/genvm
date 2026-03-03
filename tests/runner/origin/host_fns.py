# This file is auto-generated. Do not edit!

from enum import IntEnum, StrEnum
import typing


class Methods(IntEnum):
	STORAGE_READ = 0
	STORAGE_WRITE = 1
	CONSUME_FUEL = 2
	ETH_CALL = 3
	GET_BALANCE = 4
	REMAINING_FUEL_AS_GEN = 5
	NOTIFY_NONDET_DISAGREEMENT = 6
	GET_LEADER_NONDET_RESULT = 7
	CONSUME_RESULT = 8


class Errors(IntEnum):
	OK = 0
	ABSENT = 1
	FORBIDDEN = 2
	I_AM_LEADER = 3
	OUT_OF_STORAGE_GAS = 4
