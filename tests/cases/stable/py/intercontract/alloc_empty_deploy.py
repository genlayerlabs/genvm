# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


# Regression guard (genvm fee-allocation item #8): DeployContract variant.
#
# An empty message-fee-allocation tree must make a deploy emission hard-trap
# (genlayer_sdk.rs DeployContract path: no matching node -> oom_trap fees().internal()),
# never silently emit nothing.
#
# Paired positive case: ../intercontract/deploy.py (default tree -> DeployContract
# emission succeeds).
class Contract(gl.contract.Contract):
	def __init__(self):
		gl.contract.deploy(code='not really a contract'.encode('utf-8'))
