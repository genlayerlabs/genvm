# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			try:
				return gl.nondet.web.render(
					'https://test-server.genlayer.com/static/genvm/not-exists', mode='text'
				)
			except gl.nondet.NondetException as e:
				print('Error!')
				print(e)

		print(repr(gl.eq_principle.strict_eq(run)))
