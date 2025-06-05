__all__ = (
	'render',
	'request',
	'get',
	'post',
	'delete',
	'head',
	'patch',
	'Response',
)

import typing

from .._internal import _lazy_api
from ...py.types import *
import io
import dataclasses

import genlayer.gl._internal.gl_call as gl_call

from . import _decode_nondet, Image


@typing.overload
def render(
	url: str,
	*,
	wait_after_loaded: str | None = None,
	mode: typing.Literal['text', 'html'],
) -> str: ...


@typing.overload
def render(
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
def get(
	url: str,
	*,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return request.lazy(url, method='GET', headers=headers)


@_lazy_api
def post(
	url: str,
	*,
	body: str | bytes | None = None,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return request.lazy(url, method='POST', body=body, headers=headers)


@_lazy_api
def delete(
	url: str,
	*,
	body: str | bytes | None = None,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return request.lazy(url, method='DELETE', body=body, headers=headers)


@_lazy_api
def head(
	url: str,
	*,
	body: str | bytes | None = None,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return request.lazy(url, method='HEAD', body=body, headers=headers)


@_lazy_api
def patch(
	url: str,
	*,
	body: str | bytes | None = None,
	headers: dict[str, str | bytes] = {},
) -> Lazy[Response]:
	return request.lazy(url, method='PATCH', body=body, headers=headers)


@_lazy_api
def request(
	url: str,
	*,
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
def render(
	url: str,
	*,
	mode: typing.Literal['html', 'text', 'screenshot'] = 'text',
	wait_after_loaded: str | None = None,
) -> Lazy[str | Image]:
	"""
	API to get a webpage after rendering it in a browser-like environment

	:param url: url of website
	:param mode: Mode in which to return the result
	:param wait_after_loaded: How long to wait after dom loaded (for js to emit dynamic content). Should be in format such as "1000ms" or "1s"
	"""

	def decoder(x):
		x = _decode_nondet(x)
		if mode != 'screenshot':
			return typing.cast(str, x['text'])
		raw = typing.cast(bytes, x['image'])
		import PIL.Image

		pil = PIL.Image.open(io.BytesIO(raw))
		return Image(raw, pil)

	return gl_call.gl_call_generic(
		{
			'WebRender': {
				'url': url,
				'mode': mode,
				'wait_after_loaded': wait_after_loaded or '0ms',
			}
		},
		decoder,
	)
