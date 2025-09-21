# { "Depends": "py-genlayer:test" }
from genlayer import *

import html
from html.parser import HTMLParser

fcf = set(['script', 'iframe'])


class ScriptRemover(HTMLParser):
	result: list[str]

	def __init__(self):
		super().__init__()
		self.result = []
		self.in_script = False

	def handle_starttag(self, tag, attrs):
		if tag.lower() in fcf:
			self.in_script = True
		elif not self.in_script:
			self.result.append(self.get_starttag_text())

	def handle_endtag(self, tag):
		if tag.lower() in fcf:
			self.in_script = False
		elif not self.in_script:
			self.result.append(f'</{tag}>')

	def handle_data(self, data):
		if not self.in_script:
			self.result.append(data)


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run() -> str:
			res = gl.nondet.web.render(
				'https://test-server.genlayer.com/static/genvm/hello.html', mode=mode
			)  # type: ignore
			if mode == 'html':
				parser = ScriptRemover()
				parser.feed(res)
				res = ''.join(parser.result)
			return res

		print(gl.eq_principle.strict_eq(run).strip())
