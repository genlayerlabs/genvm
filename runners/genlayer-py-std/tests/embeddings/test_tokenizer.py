from word_piece_tokenizer import WordPieceTokenizer
from transformers import AutoTokenizer
import pytest

genvm_tokenizer = WordPieceTokenizer()
hug_tokenzier = AutoTokenizer.from_pretrained('sentence-transformers/all-MiniLM-L6-v2')


@pytest.mark.parametrize(
	'txt',
	[
		'this is an example sentence',
		'This is also an example sentence. But with Upper Letters.',
	],
)
def test_is_same(txt: str):
	data_got = genvm_tokenizer.tokenize(txt)
	data_exp = hug_tokenzier(txt)['input_ids']
	assert data_got == data_exp
