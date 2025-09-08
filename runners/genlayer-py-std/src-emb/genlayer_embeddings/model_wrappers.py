__all__ = ('Model', 'SentenceTransformer')

import numpy as np
from numpy.typing import DTypeLike
from ._nn.builder import Builder
from ._nn import get_run_onnx
from pathlib import Path
import json
import onnx
import collections.abc
import typing
import os
import warnings

_models = os.getenv('GENLAYER_EMBEDDINGS_MODELS', '')
_models_paths = _models.split(':')

_ALL_MODELS = {}

for i in _models_paths:
	if len(i) == 0:
		continue
	p = Path(i)
	data = json.loads(p.joinpath('model.json').read_text())
	_ALL_MODELS[data['name']] = {'path': p.joinpath('model.onnx'), **data}


# type Model = typing.Callable[..., dict[str, np.ndarray]]


def get_model(model: str, inputs: dict[str, DTypeLike], *, models_db=_ALL_MODELS):
	model_desc = models_db[model]

	# Create input placeholders as variable names
	user_inputs = {}
	for k in inputs.keys():
		user_inputs[k] = k

	onnx_model = onnx.load_model(model_desc['path'], load_external_data=False)
	rename_outputs = model_desc.get('rename-outputs', {})
	builder, inp = get_run_onnx(onnx_model, user_inputs, rename_outputs)

	builder._prelude.append(
		f'tokens_truncate = {repr(model_desc.get("tokens_truncate", None))}\n'
	)

	return builder.finish(parameters=inp)


def prod(x: collections.abc.Sequence[int]):
	res = 1
	for i in x:
		res *= i
	return res


def _unfold(x: np.ndarray):
	return x.reshape(prod(x.shape))


_cache_SentenceTransformer: dict[str, typing.Callable[[str], np.ndarray]] = {}


def SentenceTransformerFromPath(path: str) -> typing.Callable[[str], np.ndarray]:
	if res := _cache_SentenceTransformer.get(path):
		return res
	from word_piece_tokenizer import WordPieceTokenizer

	tokenizer = WordPieceTokenizer()
	data = Path(path).read_text()
	globs = {}
	exec(data, globs)
	nn_model = globs['main']

	truncate: int | None = globs.get('tokens_truncate', None)

	def ret(text: str) -> np.ndarray:
		res = tokenizer.tokenize(text)
		if truncate and len(res) > truncate:
			warnings.warn(f'truncating input tokens from {len(res)} to {truncate}')
			res = res[:truncate]
		res = np.array(res, np.int64)
		res = res.reshape(1, prod(res.shape))
		return _unfold(
			nn_model(
				input_ids=res,
				attention_mask=np.zeros(res.shape, res.dtype),
				token_type_ids=np.zeros(res.shape, res.dtype),
			)['embedding']
		)

	_cache_SentenceTransformer[path] = ret

	return ret


def SentenceTransformer(model: str) -> typing.Callable[[str], np.ndarray]:
	if res := _cache_SentenceTransformer.get(model):
		return res
	from word_piece_tokenizer import WordPieceTokenizer

	tokenizer = WordPieceTokenizer()
	nn_model = get_model(
		model,
		{
			'input_ids': np.int64,
			'attention_mask': np.int64,
			'token_type_ids': np.int64,
		},
	)

	model_desc = _ALL_MODELS[model]
	truncate: int | None = model_desc.get('tokens_truncate', None)

	def ret(text: str) -> np.ndarray:
		res = tokenizer.tokenize(text)
		if truncate and len(res) > truncate:
			warnings.warn(f'truncating input tokens from {len(res)} to {truncate}')
			res = res[:truncate]
		res = np.array(res, np.int64)
		res = res.reshape(1, prod(res.shape))
		return _unfold(
			nn_model(
				input_ids=res,
				attention_mask=np.zeros(res.shape, res.dtype),
				token_type_ids=np.zeros(res.shape, res.dtype),
			)['embedding']
		)

	_cache_SentenceTransformer[model] = ret

	return ret
