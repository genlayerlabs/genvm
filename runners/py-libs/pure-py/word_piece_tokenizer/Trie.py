from typing import Union


class Trie:

    def __init__(self, unk_token_id):
        self.data = {}
        self.unk_token_id = unk_token_id

    def add(self, token: str, token_id: str):
        if not token:
            return
        ref = self.data
        for char in token:
            ref[char] = char in ref and ref[char] or {}
            ref = ref[char]
        ref[""] = token_id

    def getLongestMatchToken(self, text: str) -> Union[int, str]:
        """Returns token of the longest match and the remaining unmatched text."""
        curr_index = longest_match_end_index = 0
        longest_match_token = -1
        ref = self.data

        while curr_index < len(text) and text[curr_index] in ref:
            ref = ref[text[curr_index]]
            if ("" in ref and (len(text) == 1 or text[curr_index] != "#")):
                longest_match_end_index = curr_index
                longest_match_token = ref[""]
            curr_index += 1
        if longest_match_token == -1:
            return self.unk_token_id, text[1:]
        else:
            return longest_match_token, text[longest_match_end_index + 1:]
