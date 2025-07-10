# v0.1.5
# { "Depends": "py-genlayer:test" }
from genlayer import *


class TestEvent(gl.Event):
	def __init__(self, user_id: int, action: str, /, **blob): ...


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		try:
			# Test basic event emission
			TestEvent(42, 'create', timestamp=1234567890, data='test_data').emit()

			# Test event with different parameters
			TestEvent(100, 'update', amount=500, description='Updated record').emit()

			print('Events emitted successfully')
		except Exception as e:
			print(f'Error emitting event: {e}')
