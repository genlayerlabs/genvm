__all__ = ('get_webpage', 'exec_prompt', 'GetWebpageKwArgs', 'ExecPromptKwArgs')

import typing
from ._internal import lazy_from_fd, _lazy_api
from ..py.types import *
import genlayer.std._wasi as wasi
import json


class GetWebpageKwArgs(typing.TypedDict):
	mode: typing.Literal['html', 'text']
	"""
	Mode in which to return the result
	"""

	wait_after_loaded: typing.NotRequired[str]
	"""
	How long to wait after dom loaded (for js to emit dynamic content)
	Should be in format such as "1000ms" or "1s"
	"""


@_lazy_api
def get_webpage(url: str, **config: typing.Unpack[GetWebpageKwArgs]) -> Lazy[str]:
	"""
	API to get a webpage after rendering it

	:param url: url of website
	:type url: ``str``

	:param \\*\\*config: configuration
	:type \\*\\*config: :py:class:`GetWebpageKwArgs`

	:rtype: ``str``
	"""
	return lazy_from_fd(
		wasi.get_webpage(json.dumps(config), url), lambda buf: str(buf, 'utf-8')
	)


class ExecPromptKwArgs(typing.TypedDict):
	pass


@_lazy_api
def exec_prompt(prompt: str, **config: typing.Unpack[ExecPromptKwArgs]) -> Lazy[str]:
	"""
	API to execute a prompt (perform NLP)

	:param prompt: prompt itself
	:type prompt: ``str``

	:param \\*\\*config: configuration
	:type \\*\\*config: :py:class:`ExecPromptKwArgs`

	:rtype: ``str``
	"""

	return lazy_from_fd(
		wasi.exec_prompt(json.dumps(config), prompt), lambda buf: str(buf, 'utf-8')
	)
