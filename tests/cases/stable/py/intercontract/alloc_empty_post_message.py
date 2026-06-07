# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


# Regression guard (genvm fee-allocation item #8).
#
# genvm is a pure CONSUMER of the message-fee-allocation tree the node provides.
# With an EMPTY tree, emitting an internal message MUST hard-trap (the find_map
# in genlayer_sdk.rs finds no matching node -> `oom_trap(... .fees().internal())`),
# it must NEVER silently emit nothing. A silent-0 here was the exact symptom of
# the consensus/node DisallowedFeeAllocation([]) bug investigated cross-repo; this
# test pins genvm's side so it can't regress to "0 emissions, no error".
#
# Paired positive case: ../intercontract/send_message.py (default wildcard tree
# -> the same emit succeeds with a PostMessage emission).
class Contract(gl.contract.Contract):
	def __init__(self):
		gl.contract.get_at(gl.Address(b'\x30' * 20)).emit().foo(1, 2)
