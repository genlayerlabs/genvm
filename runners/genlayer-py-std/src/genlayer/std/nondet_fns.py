__all__ = (
	'get_webpage',
	'exec_prompt',
	'GetWebpageKwArgs',
	'ExecPromptKwArgs',
	'request',
	'Response',
)

import typing

from ._internal import _lazy_api
from ..py.types import *
import genlayer.std._wasi as wasi
import io
import dataclasses

import genlayer.std._internal.gl_call as gl_call


class NondetException(Exception):
	""" """


class GetWebpageKwArgs(typing.TypedDict):
	mode: typing.Literal['html', 'text', 'screenshot']
	"""
	Mode in which to return the result
	"""

	wait_after_loaded: typing.NotRequired[str]
	"""
	How long to wait after dom loaded (for js to emit dynamic content)
	Should be in format such as "1000ms" or "1s"
	"""


import genlayer.py.calldata as calldata


def _decode_nondet(buf):
	ret = typing.cast(dict, calldata.decode(buf))
	if err := ret.get('error'):
		raise NondetException(err)
	return ret['ok']


if typing.TYPE_CHECKING:
	import PIL.Image


@dataclasses.dataclass
class Image:
	raw: bytes
	pil: 'PIL.Image.Image'


@typing.overload
def get_webpage(
	url: str,
	*,
	wait_after_loaded: str | None = None,
	mode: typing.Literal['text', 'html'],
) -> str: ...


@typing.overload
def get_webpage(
	url: str, *, wait_after_loaded: str | None = None, mode: typing.Literal['screenshot']
) -> Image: ...


@dataclasses.dataclass
class Response:
	status: int
	headers: dict[str, bytes]
	body: bytes | None


def str_or_bytes_to_bytes(
	data: str | bytes | None,
) -> bytes | None:
	if data is None:
		return None
	if isinstance(data, str):
		return data.encode('utf-8')
	return data


@_lazy_api
def request(
	url: str,
	method: typing.Literal['GET', 'POST', 'DELETE', 'HEAD', 'OPTIONS', 'PATCH'],
	body: str | bytes | None = None,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return gl_call.gl_call_generic(
		{
			'WebRequest': {
				'url': url,
				'method': method,
				'body': str_or_bytes_to_bytes(body),
				'headers': {k: str_or_bytes_to_bytes(v) for k, v in headers.items()},
			}
		},
		lambda x: Response(**(_decode_nondet(x)['response'])),
	)


@_lazy_api
def get_webpage(
	url: str, **config: typing.Unpack[GetWebpageKwArgs]
) -> Lazy[str | Image]:
	"""
	API to get a webpage after rendering it

	:param url: url of website
	:type url: ``str``

	:param \\*\\*config: configuration
	:type \\*\\*config: :py:class:`GetWebpageKwArgs`

	:rtype: ``str``
	"""

	def decoder(x):
		x = _decode_nondet(x)
		if config.get('mode', 'text') != 'screenshot':
			return typing.cast(str, x['text'])
		raw = typing.cast(bytes, x['image'])
		import PIL.Image

		pil = PIL.Image.open(io.BytesIO(raw))
		return Image(raw, pil)

	return gl_call.gl_call_generic(
		{
			'WebRender': {
				'url': url,
				'mode': config.get('mode', 'text'),
				'wait_after_loaded': config.get('wait_after_loaded', '0ms'),
			}
		},
		decoder,
	)


class ExecPromptKwArgs(typing.TypedDict):
	response_format: typing.NotRequired[typing.Literal['text', 'json']]
	"""
	Defaults to ``text``
	"""
	images: typing.NotRequired[collections.abc.Sequence[bytes | Image] | None]


@typing.overload
def exec_prompt(
	prompt: str, *, images: collections.abc.Sequence[bytes | Image] | None = None
) -> str: ...


@typing.overload
def exec_prompt(
	prompt: str,
	*,
	response_format: typing.Literal['text'],
	images: collections.abc.Sequence[bytes | Image] | None = None,
) -> str: ...


@typing.overload
def exec_prompt(
	prompt: str,
	*,
	response_format: typing.Literal['json'],
	image: bytes | Image | None = None,
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

	images: list[bytes] = []
	for im in config.get('images', None) or []:
		if isinstance(im, Image):
			images.append(im.raw)
		else:
			images.append(im)

	return gl_call.gl_call_generic(
		{
			'ExecPrompt': {
				'prompt': prompt,
				'response_format': config.get('response_format', 'text'),
				'images': images,
			}
		},
		_decode_nondet,
	)
