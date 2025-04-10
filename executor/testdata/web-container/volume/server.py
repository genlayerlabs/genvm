import http.server as httpserv
import time


class MyHTTPHandler(httpserv.SimpleHTTPRequestHandler):
	def __init__(self, *args, **kwargs):
		httpserv.SimpleHTTPRequestHandler.__init__(
			self, *args, **kwargs, directory='/test/http'
		)

	def do_GET(self):
		if 'stuck4timeout' in self.path:
			time.sleep(10)
		super().do_GET()


serv = httpserv.ThreadingHTTPServer(('127.0.0.1', 80), MyHTTPHandler)
serv.serve_forever()
