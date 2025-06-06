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

		if '/status/200' in self.path:
			self.send_response(200)
			self.end_headers()
			self.wfile.write(b'OK')
			return
		elif 'body' in self.path:
			self.send_response(200)
			self.end_headers()
			self.wfile.write(b'\xde\xad\xbe\xef')
			return
		elif '/status/404' in self.path:
			self.send_response(404)
			self.end_headers()
			self.wfile.write(b'Not Found')
			return

		super().do_GET()


serv = httpserv.ThreadingHTTPServer(('0.0.0.0', 80), MyHTTPHandler)
serv.serve_forever()
