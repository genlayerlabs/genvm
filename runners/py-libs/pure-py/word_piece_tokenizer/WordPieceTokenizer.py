from collections import OrderedDict
from pathlib import Path
import os

from .BasicTokenizer import BasicTokenizer
from .Trie import Trie
from .utils import load_vocab


class WordPieceTokenizer:

    def __init__(self):
        self.unk_token = "[UNK]"
        self.sep_token = "[SEP]"
        self.cls_token = "[CLS]"

        self.vocab = load_vocab(os.path.join(str(Path(__file__).resolve().parent), "vocab.txt"))
        self._initialise_tokens_trie()

        self.ids_to_tokens = OrderedDict([(ids, tok)
                                          for tok, ids in self.vocab.items()])
        self.basic_tokenizer = BasicTokenizer()

    def _initialise_tokens_trie(self):
        self.tokens_trie = Trie(self._convert_token_to_id(self.unk_token))
        for tok, tok_id in self.vocab.items():
            self.tokens_trie.add(tok, tok_id)

    def tokenize(self, text):
        cls_token_id = self._convert_token_to_id(self.cls_token)
        sep_token_id = self._convert_token_to_id(self.sep_token)
        tokenized_text = [cls_token_id, *self._tokenize(text), sep_token_id]
        return tokenized_text

    def _tokenize(self, text):
        split_tokens = []
        for token in self.basic_tokenizer.tokenize(text):
            split_tokens += self._wordpiece_tokenize(token)
        return split_tokens

    def _wordpiece_tokenize(self, text):
        token_ids = []

        while text != "##":
            token_id, text = self.tokens_trie.getLongestMatchToken(text)
            text = "##" + text
            token_ids = [*token_ids, token_id]

        return token_ids

    def _convert_token_to_id(self, token):
        return self.vocab.get(token, self.vocab.get(self.unk_token))

    def _convert_id_to_token(self, index):
        return self.ids_to_tokens.get(index, self.unk_token)

    def convert_tokens_to_string(self, tokens):
        out_string = " ".join(tokens).replace(" ##", "").strip()
        return out_string

    def convert_ids_to_tokens(self, ids):
        return [self._convert_id_to_token(tok_id) for tok_id in ids]
