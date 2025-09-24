#!/usr/bin/env python3

import http.server
import io
import re
import socketserver
import argparse
import signal
import sys
import json
import time, datetime
import typing
from pathlib import Path
import traceback
import subprocess
import threading
from urllib.parse import urlparse

ROOT_PATH = Path(__file__).parent.parent
_log_lock = threading.Lock()

version_re = re.compile(r'^v(\d+|\*)\.(\d+|\*)\.(\d+|\*)$')


def get_version_parts(version: str) -> typing.Tuple[int | None, int | None, int | None]:
	m = version_re.match(version)
	if not m:
		raise ValueError(f'Invalid version format: {version}')
	major, minor, patch = m.groups()
	return (
		int(major) if major != '*' else None,
		int(minor) if minor != '*' else None,
		int(patch) if patch != '*' else None,
	)


def _log_value_unfold(x):
	if isinstance(x, Exception):
		return {
			'type': type(x).__name__,
			'message': str(x),
			'args': list(x.args) if x.args else [],
			'notes': getattr(x, '__notes__', []),
			'traceback': traceback.format_exception(type(x), x, x.__traceback__),
		}
	if isinstance(x, datetime.datetime):
		return x.isoformat()
	return x


def log_json(level, message, **kwargs):
	"""Log JSON formatted message to stdout"""
	log_entry = {'timestamp': time.time(), 'level': level, 'message': message, **kwargs}

	with _log_lock:
		print(json.dumps(log_entry, default=_log_value_unfold), flush=True)


def handle_detect_version(handler: http.server.BaseHTTPRequestHandler) -> str:
	"""Handle /detect-version endpoint"""
	deployment_timestamp = handler.headers['Deployment-Timestamp']

	# Read request body
	content_length = int(handler.headers.get('Content-Length', 0))
	body = handler.rfile.read(content_length)

	deployment_dt = datetime.datetime.fromisoformat(deployment_timestamp)

	# Find maximum available version at deployment time
	max_deployment_version: str | None = None
	max_timestamp = None

	for version, timestamp in all_versions.items():
		if timestamp <= deployment_dt:
			if max_timestamp is None or timestamp > max_timestamp:
				max_deployment_version = version
				max_timestamp = timestamp

	if max_deployment_version is None:
		raise ValueError('No suitable version found for deployment timestamp')

	log_json(
		'debug',
		'Detected max version at deployment time',
		deployment_timestamp=deployment_timestamp,
		version=max_deployment_version,
	)

	executor = ROOT_PATH.joinpath('executor', max_deployment_version, 'bin', 'genvm')

	try:
		subproc = subprocess.run(
			[executor, 'parse-version'],
			check=True,
			capture_output=True,
			text=True,
			stdin=io.BytesIO(body),
		)
	except Exception as e:
		# if we fail to detect version, then return this executor so that it will report the same error
		# when ran
		return json.dumps(
			{'version': max_deployment_version, 'version_pattern': None, 'error': str(e)}
		)

	ver = subproc.stdout.strip()
	log_json('debug', 'GenVM reported version', version=ver)

	s_major, s_minor, _patch = get_version_parts(ver)

	result_version = None
	for version, timestamp in all_versions.items():
		if timestamp > max_timestamp:
			continue
		v_compare_to = get_version_parts(version)
		c_major, c_minor, _v_patch = v_compare_to
		assert c_major is not None
		assert c_minor is not None

		if s_major is not None and c_major != s_major:
			continue
		if s_minor is not None and c_minor != s_minor:
			continue
		if c_minor is not None and s_minor is not None and c_minor != s_minor:
			continue
		if result_version is None or v_compare_to > result_version:
			result_version = v_compare_to

	if result_version is None:
		result_version = max_deployment_version
	response_data = {'version': result_version, 'version_pattern': ver, 'error': None}
	response_json = json.dumps(response_data)

	return response_json


all_versions = json.loads(
	ROOT_PATH.joinpath('data', 'version-timestamps.json').read_text()
)
for k in all_versions:
	all_versions[k] = datetime.datetime.fromisoformat(all_versions[k])

log_json('info', 'Loaded version timestamps', versions=all_versions)


class DetectVersionHandler(http.server.BaseHTTPRequestHandler):
	def do_POST(self):
		"""Handle POST requests"""
		start_time = time.time()

		if self.path == '/detect-version':
			try:
				result = handle_detect_version(self)
			except Exception as e:
				log_json(
					'error',
					'POST request failed',
					path=self.path,
					client_ip=self.client_address[0],
					status_code=200,
					error=e,
				)
				self.send_response(500)
				self.send_header('Content-type', 'plain/text')
				self.end_headers()
				self.wfile.write(str(e).encode('utf-8'))
			else:
				self.send_response(200)
				self.send_header('Content-type', 'application/json')
				self.end_headers()
				self.wfile.write(result.encode('utf-8'))

			log_json(
				'info',
				'POST request processed',
				path=self.path,
				client_ip=self.client_address[0],
				status_code=200,
				response_time_ms=round((time.time() - start_time) * 1000, 2),
			)
		else:
			self.send_error(404)
			log_json(
				'warning',
				'404 Not Found',
				path=self.path,
				client_ip=self.client_address[0],
				status_code=404,
			)

	def log_message(self, format, *args):
		log_json('debug', 'http log', submessage=format % args)


def signal_handler(signum, frame):
	"""Handle keyboard interrupt"""
	log_json('info', 'Shutting down server')
	sys.exit(0)


def main():
	parser = argparse.ArgumentParser(description='Detect version server')
	parser.add_argument('--port', type=int, default=8080, help='Port to listen on')
	parser.add_argument('--host', type=str, default='localhost', help='Host to listen on')
	args = parser.parse_args()

	# Set up signal handler for graceful shutdown
	signal.signal(signal.SIGINT, signal_handler)

	log_json('info', 'Starting server', host=args.host, port=args.port)

	with socketserver.TCPServer((args.host, args.port), DetectVersionHandler) as httpd:
		log_json('info', 'Server ready', url=f'http://{args.host}:{args.port}')
		try:
			httpd.serve_forever()
		except KeyboardInterrupt:
			log_json('info', 'Server stopped')


if __name__ == '__main__':
	main()
