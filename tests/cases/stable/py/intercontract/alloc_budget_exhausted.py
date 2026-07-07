# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


# Regression guard (genvm fee-allocation item #8): node MATCHES but its budget is
# too small.
#
# Here the allocation tree has a matching wildcard internal-finalized node, so the
# find_map succeeds, but its `budget` is below the computed message_fee. genvm must
# hard-trap in consume_message_fee_internal ("message fee cost exceeds node budget"
# -> oom_trap fees().internal()), never silently emit. This pins the second trap
# gate (budget), distinct from the no-matching-node gate.
class Contract(gl.contract.Contract):
	def __init__(self):
		gl.contract.get_at(gl.Address(b'\x30' * 20)).emit().foo(1, 2)
