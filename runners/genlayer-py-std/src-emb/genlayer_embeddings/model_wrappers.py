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

_models = os.getenv('GENLAYER_EMBEDDINGS_MODELS', '')
_models_paths = _models.split(':')

_ALL_MODELS = {}

for i in _models_paths:
	if len(i) == 0:
		continue
	p = Path(i)
	data = json.loads(p.joinpath('model.json').read_text())
	_ALL_MODELS[data['name']] = {'path': p.joinpath('model.onnx'), **data}


class Model:
	_compiled_model: typing.Callable
	_input_names: list[str]
	_output_names: list[str]

	def __init__(
		self, model: str, inputs: dict[str, DTypeLike], *, models_db=_ALL_MODELS
	):
		model_desc = models_db[model]
		onnx_model = onnx.load_model(model_desc['path'], load_external_data=False)

		# Create input placeholders as variable names
		user_inputs = {}
		self._input_names = list(inputs.keys())
		for k in inputs.keys():
			user_inputs[k] = k

		self._compiled_model = get_run_onnx(onnx_model, user_inputs)

		# Extract output names from the ONNX model
		self._output_names = [output.name for output in onnx_model.graph.output]

		# Apply renaming if specified
		rename_outputs = model_desc.get('rename-outputs', {})
		self._output_names = [rename_outputs.get(name, name) for name in self._output_names]

	def __call__(
		self, inputs: dict[str, np.ndarray], outputs: list[str] | None = None
	) -> dict[str, np.ndarray]:
		if outputs is None:
			outputs = self._output_names

		# Execute the compiled model with the inputs
		result_values = self._compiled_model(**inputs)

		# If there's only one output, wrap in tuple
		if not isinstance(result_values, tuple):
			result_values = (result_values,)

		# Map outputs to their names
		result = {}
		for i, output_name in enumerate(self._output_names):
			if output_name in outputs and i < len(result_values):
				result[output_name] = result_values[i]

		return result


def prod(x: collections.abc.Sequence[int]):
	res = 1
	for i in x:
		res *= i
	return res


def _unfold(x: np.ndarray):
	return x.reshape(prod(x.shape))


_cache_SentenceTransformer: dict[str, typing.Callable[[str], np.ndarray]] = {}


def SentenceTransformer(model: str) -> typing.Callable[[str], np.ndarray]:
	if res := _cache_SentenceTransformer.get(model):
		return res
	from word_piece_tokenizer import WordPieceTokenizer

	tokenizer = WordPieceTokenizer()
	nn_model = Model(
		model,
		{
			'input_ids': np.int64,
			'attention_mask': np.int64,
			'token_type_ids': np.int64,
		},
	)

	def ret(text: str) -> np.ndarray:
		res = tokenizer.tokenize(text)
		res = np.array(res, np.int64)
		res = res.reshape(1, prod(res.shape))
		return _unfold(
			nn_model(
				{
					'input_ids': res,
					'attention_mask': np.zeros(res.shape, res.dtype),
					'token_type_ids': np.zeros(res.shape, res.dtype),
				},
				outputs=['embedding'],
			)['embedding']
		)

	_cache_SentenceTransformer[model] = ret

	return ret
