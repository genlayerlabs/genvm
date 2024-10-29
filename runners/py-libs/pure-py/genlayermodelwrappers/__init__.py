__all__ = ('Model',)

import numpy as np
from numpy.typing import DTypeLike
from genlayer.py.nn.tensor import Tensor, InputTensor, TensorStorage
from genlayer.py.nn import get_run_onnx
from pathlib import Path
import json
import onnx
import collections.abc
import typing

_ALL_MODELS = json.loads(Path(__file__).parent.joinpath('models.json').read_text())

class Model:
	_inputs: dict[str, InputTensor]
	_outputs: dict[str, Tensor]
	_store: TensorStorage

	def __init__(self, model: str, inputs: dict[str, DTypeLike], *, models_db=_ALL_MODELS):
		model_desc = models_db[model]
		self._store = TensorStorage()
		onnx_model = onnx.load_model(model_desc['path'], load_external_data=False)
		self._inputs = {
			k: self._store.input(None, v)
			for k, v in inputs.items()
		}
		res = get_run_onnx(onnx_model, typing.cast(dict[str, Tensor], self._inputs), self._store)
		self._outputs = {
			model_desc.get('rename-outputs', {}).get(k, k): v
			for k, v in res.items()
		}

		self._store.finish()

	def __call__(self, inputs: dict[str, np.ndarray], outputs: list[str] | None = None) -> dict[str, np.ndarray]:
		if outputs is None:
			outputs = list(self._outputs.keys())

		self._store.reset()
		for k, v in inputs.items():
			self._inputs[k].set_input(v)



		return {
			v: self._outputs[v].compute()
			for v in outputs
		}

def prod(x: collections.abc.Sequence[int]):
	res = 1
	for i in x:
		res *= i
	return res

def SentenceTransformer(model: str) -> typing.Callable[[str], np.ndarray]:
	from word_piece_tokenizer import WordPieceTokenizer
	tokenizer = WordPieceTokenizer()
	nn_model = Model(model, {
		'input_ids': np.int64,
		'attention_mask': np.int64,
		'token_type_ids': np.int64,
	})
	def ret(text: str) -> np.ndarray:
		res = tokenizer.tokenize(text)
		res = np.array(res, np.int64)
		res = res.reshape(1, prod(res.shape))
		return nn_model({
			'input_ids': res,
			'attention_mask': np.zeros(res.shape, res.dtype),
			'token_type_ids': np.zeros(res.shape, res.dtype),
		},
		outputs=['embedding'])['embedding']

	return ret
