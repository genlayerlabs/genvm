__all__ = ('get_webpage', 'exec_prompt', 'GetWebpageKwArgs', 'ExecPromptKwArgs')

import typing
from ._internal import _lazy_api
from ..py.types import *
import genlayer.std._wasi as wasi
import json

import genlayer.std._internal.gl_call as gl_call


class NondetException(Exception):
	""" """


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


def _decode_nondet(buf):
	ret = json.loads(bytes(buf).decode('utf-8'))
	if err := ret.get('error'):
		raise NondetException(err)
	return ret['ok']


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

	return gl_call.gl_call_generic(
		{
			'WebRender': {
				'url': url,
				'mode': config.get('mode', 'text'),
				'wait_after_loaded': config.get('wait_after_loaded', '0ms'),
			}
		},
		lambda x: _decode_nondet(x)['text'],  # in future we may add images here as well
	)


class ExecPromptKwArgs(typing.TypedDict):
	response_format: typing.NotRequired[typing.Literal['text', 'json']]
	"""
	Defaults to ``text``
	"""
	image: typing.NotRequired[bytes | None]


@typing.overload
def exec_prompt(prompt: str, *, image: bytes | None = None) -> str: ...


@typing.overload
def exec_prompt(
	prompt: str, *, response_format: typing.Literal['text'], image: bytes | None = None
) -> str: ...


@typing.overload
def exec_prompt(
	prompt: str, *, response_format: typing.Literal['json'], image: bytes | None = None
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

	return gl_call.gl_call_generic(
		{
			'ExecPrompt': {
				'prompt': prompt,
				'response_format': config.get('response_format', 'text'),
				'image': config.get('image', None),
			}
		},
		_decode_nondet,
	)
