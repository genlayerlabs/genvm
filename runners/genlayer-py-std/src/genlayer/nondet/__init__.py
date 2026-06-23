"""
Non-deterministic operations module.

This module provides APIs for operations that may produce different results
across different nodes, such as:
- ``exec_prompt``: Execute LLM prompts
- ``web``: Web interaction functionality
- ``Image``: Image dataclass for multimodal prompts
"""

__all__ = (
	'web',
	'exec_prompt',
	'Image',
)

import typing
import collections.abc

from genlayer._internal import _lazy_api
from genlayer.types import *
import _genlayer_wasi as wasi
import json
import dataclasses

import genlayer._internal.on_chain.gl_call as gl_call


class NondetException(Exception):
	causes: list[str]
	ctx: dict[str, typing.Any]

	def __init__(self, causes: list[str], ctx: dict[str, typing.Any]):
		self.causes = causes
		self.ctx = ctx
		super().__init__(': '.join(causes))


import genlayer.calldata as calldata


def _decode_nondet(buf):
	ret = typing.cast(dict, calldata.decode(buf))
	if err := ret.get('error'):
		if isinstance(err, dict) and 'causes' in err:
			raise NondetException(
				causes=err.get('causes', []),
				ctx=err.get('ctx', {}),
			)
		raise NondetException(causes=[str(err)], ctx={})
	return ret['ok']


def _decode_nondet_json(buf):
	return json.loads(_decode_nondet(buf))


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
	prompt: str, /, **config: typing.Unpack[ExecPromptKwArgs]
) -> Lazy[str | dict]:
	"""
	API to execute a prompt (perform NLP)

	:param prompt: prompt itself
	:type prompt: ``str``

	:param \\*\\*config: configuration
	:type \\*\\*config: :py:class:`ExecPromptKwArgs`

	:rtype: ``str``
	"""

	if len(prompt) == 0:
		raise ValueError('Prompt cannot be empty')

	images: list[bytes] = []
	for im in config.get('images', None) or []:
		if isinstance(im, Image):
			images.append(im.raw)
		else:
			images.append(im)

	format = config.get('response_format', 'text')

	return gl_call.gl_call_generic(
		{
			'ExecPrompt': {
				'prompt': prompt,
				'response_format': format,
				'images': images,
			}
		},
		_decode_nondet_json if format == 'json' else _decode_nondet,
	)


import genlayer.nondet.web as web
