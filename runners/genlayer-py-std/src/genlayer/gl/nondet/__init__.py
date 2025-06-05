__all__ = (
	'web',
	'exec_prompt',
	'Image',
)

import typing

from .._internal import _lazy_api
from ...py.types import *
import _genlayer_wasi as wasi
import io
import dataclasses

import genlayer.gl._internal.gl_call as gl_call


class NondetException(Exception):
	""" """


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


import genlayer.gl.nondet.web as web
