__all__ = ('get_webpage', 'exec_prompt')

import typing
from ._internal import lazy_from_fd, _LazyApi
from ..py.types import *
import genlayer.std._wasi as wasi
import json


class _GetWebpageConfig(typing.TypedDict):
	mode: typing.Literal['html', 'text']


def _get_webpage(url: str, **config: typing.Unpack[_GetWebpageConfig]) -> Lazy[str]:
	return lazy_from_fd(
		wasi.get_webpage(json.dumps(config), url), lambda buf: str(buf, 'utf-8')
	)


get_webpage = _LazyApi(_get_webpage)
"""
API to get a webpage after rendering it
"""
del _get_webpage


class _ExecPromptConfig(typing.TypedDict):
	pass


def _exec_prompt(prompt: str, **config: typing.Unpack[_ExecPromptConfig]) -> Lazy[str]:
	return lazy_from_fd(
		wasi.exec_prompt(json.dumps(config), prompt), lambda buf: str(buf, 'utf-8')
	)


exec_prompt = _LazyApi(_exec_prompt)
"""
API to execute a prompt (perform NLP)
"""
del _exec_prompt
