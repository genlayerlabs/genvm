#!/usr/bin/env python3

import asyncio
import os
import aiohttp
import time
import sys
from pathlib import Path

CACHE_FILE = Path('/tmp/healthcheck_cache')
CACHE_DURATION = 60  # seconds


def is_cache_valid():
	"""Check if the cache is valid (successful check within last minute)"""
	if not CACHE_FILE.exists():
		return False
	try:
		with CACHE_FILE.open('r') as f:
			last_success = float(f.read().strip())
		return time.time() - last_success < CACHE_DURATION
	except (ValueError, IOError):
		return False


def update_cache(success: bool):
	"""Update cache with current timestamp if successful, remove if failed"""
	if success:
		with open(CACHE_FILE, 'w') as f:
			f.write(str(time.time()))
	elif CACHE_FILE.exists():
		CACHE_FILE.unlink(missing_ok=True)


async def perform_healthcheck():
	async with aiohttp.ClientSession() as session:
		port = int(os.getenv('PORT', '4444'))
		async with session.get(
			f'http://localhost:{port}/render?mode=text&url=https%3A%2F%2Ftest-server.genlayer.com%2Fstatic%2Fgenvm%2Fhello.html'
		) as response:
			print(response.status)
			body = await response.text()
			body = body.strip().lower()
			print(body)
			if body != 'hello world!':
				raise ValueError('unexpected body: ' + body)


async def main():
	if is_cache_valid():
		print('Healthcheck cache valid, skipping')
		sys.exit(0)
	try:
		await perform_healthcheck()
		update_cache(True)
	except BaseException:
		update_cache(False)
		raise


asyncio.run(main())
