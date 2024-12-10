__all__ = ('get_webpage', 'exec_prompt', 'GetWebpageKwArgs', 'ExecPromptKwArgs')

import typing
from ._internal import lazy_from_fd, _LazyApi
from ..py.types import *
import genlayer.std._wasi as wasi
import json


class GetWebpageKwArgs(typing.TypedDict):
	mode: typing.Literal['html', 'text']
	"""
	Mode in which to return the result
	"""


def _get_webpage(url: str, **config: typing.Unpack[GetWebpageKwArgs]) -> Lazy[str]:
	return lazy_from_fd(
		wasi.get_webpage(json.dumps(config), url), lambda buf: str(buf, 'utf-8')
	)


get_webpage = _LazyApi(_get_webpage)
"""
API to get a webpage after rendering it

:param url: url of website
:type url: ``str``

:param \\*\\*config: configuration
:type \\*\\*config: :py:type:`GetWebpageKwArgs`

:rtype: ``str``

.. note::
	supports ``.lazy()`` version
"""
del _get_webpage


class ExecPromptKwArgs(typing.TypedDict):
	pass


def _exec_prompt(prompt: str, **config: typing.Unpack[ExecPromptKwArgs]) -> Lazy[str]:
	return lazy_from_fd(
		wasi.exec_prompt(json.dumps(config), prompt), lambda buf: str(buf, 'utf-8')
	)


exec_prompt = _LazyApi(_exec_prompt)
"""
API to execute a prompt (perform NLP)

:param prompt: prompt itself
:type prompt: ``str``

:param \\*\\*config: configuration
:type \\*\\*config: :py:type:`ExecPromptKwArgs`

:rtype: ``str``

.. note::
	supports ``.lazy()`` version
"""
del _exec_prompt
