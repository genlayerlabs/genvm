from word_piece_tokenizer import WordPieceTokenizer
from transformers import AutoModel
import pytest
import numpy as np
import onnx
import torch

from genlayer.py._nn import *

from . import root_dir

onnx_model_path = root_dir.joinpath('runners', 'onnx-models', 'all-MiniLM-L6-v2.onnx')
onnx_model = onnx.load_model(onnx_model_path, load_external_data=False)

genvm_tokenizer = WordPieceTokenizer()

tensor_store = TensorStorage()
input_ids = tensor_store.input(None, np.int64)
attention_mask = tensor_store.input(input_ids.shape, input_ids.dtype)
token_type_ids = tensor_store.input(input_ids.shape, input_ids.dtype)

genvm_model_out_tensors = get_run_onnx(
	onnx_model,
	{
		'input_ids': input_ids,
		'attention_mask': attention_mask,
		'token_type_ids': token_type_ids,
	},
	tensor_store,
)

genvm_model_out_tensor = genvm_model_out_tensors['last_hidden_state']
genvm_model_out_pooler_tensor = genvm_model_out_tensors['924']

tensor_store.finish()

hug_model = AutoModel.from_pretrained('sentence-transformers/all-MiniLM-L6-v2')

import collections.abc


def prod(x: collections.abc.Sequence[int]):
	res = 1
	for i in x:
		res *= i
	return res


@pytest.mark.parametrize(
	'txt',
	[
		'this is an example sentence',
		'This is also an example sentence. But with Upper Letters.',
	],
)
def test_is_same(txt: str):
	data_got = genvm_tokenizer.tokenize(txt)

	data_got = np.array(data_got, dtype=np.int64)
	data_got = data_got.reshape((1, prod(data_got.shape)))

	tensor_store.reset()
	input_ids.set_input(data_got)
	attention_mask.set_input(np.ones(data_got.shape, data_got.dtype))
	token_type_ids.set_input(np.zeros(data_got.shape, data_got.dtype))
	emb1 = genvm_model_out_tensor.compute()

	emb2_all = hug_model(
		input_ids=torch.tensor(data_got, dtype=torch.int64),
		attention_mask=torch.tensor(attention_mask.compute(), dtype=torch.int64),
		token_type_ids=torch.tensor(token_type_ids.compute(), dtype=torch.int64),
	)
	emb2 = emb2_all['last_hidden_state'].detach().numpy()

	def tst_close(x, y):
		def measure(x):
			return (x * x).sum()

		x_measure = measure(x)
		y_measure = measure(y)

		min_measure = min(x_measure, y_measure)
		diff_measure = measure(x_measure - y_measure)

		print(diff_measure)
		print(diff_measure / min_measure)

		assert diff_measure < 1e-5
		assert diff_measure / min_measure < 1e-7

	tst_close(emb1, emb2)

	pooled_output1 = genvm_model_out_pooler_tensor.compute()
	pooled_output2 = emb2_all['pooler_output'].detach().numpy()

	tst_close(pooled_output1, pooled_output2)
