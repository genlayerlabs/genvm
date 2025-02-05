from word_piece_tokenizer import WordPieceTokenizer
from transformers import AutoModel
import pytest
import numpy as np
import onnx
import torch

from genlayer.py._nn import *

from . import root_dir

onnx_model_path = root_dir.joinpath(
	*'runners/onnx-models/all-MiniLM-L6-v2/model.onnx'.split('/')
)
onnx_model = onnx.load_model(onnx_model_path, load_external_data=False)

genvm_tokenizer = WordPieceTokenizer()

tensor_store = TensorStorage()
input_ids = tensor_store.input((1, 5), np.int64)
attention_mask = tensor_store.input(input_ids.shape, input_ids.dtype)
token_type_ids = tensor_store.input(input_ids.shape, input_ids.dtype)

genvm_model_out_tensor = get_run_onnx(
	onnx_model,
	{
		'input_ids': input_ids,
		'attention_mask': attention_mask,
		'token_type_ids': token_type_ids,
	},
	tensor_store,
)['last_hidden_state']

tensor_store.finish()

hug_model = AutoModel.from_pretrained('sentence-transformers/all-MiniLM-L6-v2')


@pytest.mark.parametrize(
	'txt',
	[
		'this is an example sentence',
		'This is also an example sentence. But with Upper Letters.',
	],
)
def test_is_same(txt: str):
	data_got = genvm_tokenizer.tokenize(txt)

	data_got[4] = data_got[-1]
	while len(data_got) > 5:
		data_got.pop()

	data_got = np.array(data_got, dtype=np.int64).reshape((1, 5))

	tensor_store.reset()
	input_ids.set_input(data_got)
	attention_mask.set_input(np.ones(data_got.shape, data_got.dtype))
	token_type_ids.set_input(np.zeros(data_got.shape, data_got.dtype))
	emb1 = genvm_model_out_tensor.compute()

	emb2 = hug_model(
		input_ids=torch.tensor(data_got, dtype=torch.int64),
		attention_mask=torch.tensor(attention_mask.compute(), dtype=torch.int64),
		token_type_ids=torch.tensor(token_type_ids.compute(), dtype=torch.int64),
	)['last_hidden_state']

	emb2 = emb2.detach().numpy()

	def measure(x):
		return (x * x).sum()

	emb1_measure = measure(emb1)
	emb2_measure = measure(emb2)

	min_measure = min(emb1_measure, emb2_measure)
	diff_measure = measure(emb2 - emb1)

	print(diff_measure)
	print(diff_measure / min_measure)

	assert diff_measure < 1e-5
	assert diff_measure / min_measure < 1e-7
