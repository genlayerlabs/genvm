from word_piece_tokenizer import WordPieceTokenizer
from transformers import AutoModel
import pytest
import numpy as np
import onnx
import torch

from genlayer_embeddings.model_wrappers import Model

from . import root_dir

onnx_model_path = root_dir.joinpath(
	*'runners/models/all-MiniLM-L6-v2/model.onnx'.split('/')
)

genvm_tokenizer = WordPieceTokenizer()

# Use the model_wrappers approach for testing
models_db = {
	'all-MiniLM-L6-v2': {
		'path': onnx_model_path,
		'name': 'all-MiniLM-L6-v2',
		'rename-outputs': {
			'last_hidden_state': 'last_hidden_state',
			'924': 'pooler_output',
		},
	}
}

genvm_model = Model(
	'all-MiniLM-L6-v2',
	{
		'input_ids': np.int64,
		'attention_mask': np.int64,
		'token_type_ids': np.int64,
	},
	models_db=models_db,
)

hug_model = AutoModel.from_pretrained('sentence-transformers/all-MiniLM-L6-v2')


@pytest.mark.parametrize(
	'txt',
	[
		'this is an example sentence',
		'This is also an example sentence. But with Upper Letters.',
		'The cat sat quietly on the windowsill watching the rain fall outside.',
		'Machine learning algorithms require large datasets to achieve optimal performance.',
		'Солнце медленно скрывалось за горизонтом, окрашивая небо в красные тона.',
		'Современные технологии значительно упрощают повседневную жизнь человека.',
		'桜の花が春風に舞い散り、美しい景色を作り出している。',
		'人工知能の発展により、多くの産業が変革を迎えています。',
		'古老的图书馆里保存着许多珍贵的历史文献。',
		'科技进步为人类社会带来了前所未有的机遇和挑战。',
	],
)
def test_is_same(txt: str):
	data_got = genvm_tokenizer.tokenize(txt)

	data_got[4] = data_got[-1]
	while len(data_got) > 5:
		data_got.pop()

	data_got = np.array(data_got, dtype=np.int64).reshape((1, 5))

	# Run the GenVM model
	genvm_outputs = genvm_model(
		{
			'input_ids': data_got,
			'attention_mask': np.ones(data_got.shape, data_got.dtype),
			'token_type_ids': np.zeros(data_got.shape, data_got.dtype),
		}
	)

	emb1 = genvm_outputs['last_hidden_state']

	# Run the HuggingFace model for comparison
	emb2 = hug_model(
		input_ids=torch.tensor(data_got, dtype=torch.int64),
		attention_mask=torch.tensor(
			np.ones(data_got.shape, data_got.dtype), dtype=torch.int64
		),
		token_type_ids=torch.tensor(
			np.zeros(data_got.shape, data_got.dtype), dtype=torch.int64
		),
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
