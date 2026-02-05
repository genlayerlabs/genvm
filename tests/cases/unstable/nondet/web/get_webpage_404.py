# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			try:
				return gl.nondet.web.render(
					'https://test-server.genlayer.com/static/genvm/not-exists', mode='text'
				)
			except gl.nondet.NondetException as e:
				print('Error!')
				print(e)

		print(repr(gl.eq_principle.strict_eq(run)))
