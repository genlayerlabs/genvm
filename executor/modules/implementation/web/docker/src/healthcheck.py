#!/usr/bin/env python3

import asyncio
import sys
import aiohttp


async def main():
	async with aiohttp.ClientSession() as session:
		async with session.get('http://localhost:4444/status') as response:
			print(response.status)
			body = await response.json()
			try:
				if body['value']['ready'] != True:
					raise Exception('Not ready')
			except Exception as e:
				print(body)
				raise


asyncio.run(main())
