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
	payload = {'url': url, **config}
	return lazy_from_fd(
		wasi.web_render(json.dumps(payload)), lambda buf: json.loads(bytes(buf))['text']
	)


class ExecPromptKwArgs(typing.TypedDict):
	response_format: typing.NotRequired[typing.Literal['text', 'json']]
	"""
	Defaults to ``text``
	"""


@typing.overload
def exec_prompt(prompt: str) -> str: ...


@typing.overload
def exec_prompt(prompt: str, response_format: typing.Literal['text']) -> str: ...


@typing.overload
def exec_prompt(
	prompt: str, response_format: typing.Literal['json']
) -> dict[str, typing.Any]: ...


@_lazy_api
def exec_prompt(
	prompt: str, **config: typing.Unpack[ExecPromptKwArgs]
) -> Lazy[str | dict]:
	"""
	API to execute a prompt (perform NLP)

	:param prompt: prompt itself
	:type prompt: ``str``

	:param \\*\\*config: configuration
	:type \\*\\*config: :py:class:`ExecPromptKwArgs`

	:rtype: ``str``
	"""

	payload = {'prompt': prompt, **config}

	return lazy_from_fd(
		wasi.exec_prompt(json.dumps(payload)), lambda buf: json.loads(bytes(buf))
	)
