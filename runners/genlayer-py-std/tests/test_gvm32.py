from genlayer.gvm32 import encode, decode

# hex, gvm32 (Crockford Base32, lowercase) — shared with executor/crates/sdk-rs/tests/gvm32.rs
CASES = [
	(
		'ab335240fd942ab8191c5e628cd4ff3903c577bda961fb75df08e0303a00527b',
		'ncsn4g7xjgnbg68wbsh8sn7z741waxxxn5gzpxez13g30eg0a9xg',
	),
	('47b2d8f260c2d48116044bc43fe3de0f', '8ysdhwk0rba825g49f23zryy1w'),
	('1f74d74729abdc08f4f84e8f7f8c808c8ed92ee5', '3xtdehs9nfe0hx7r9t7qz340hj7djbq5'),
	(
		'99a2da84cec54d17325bcee0a079669c1b15eb7ead32246514b75b97862f1e00',
		'k6hdn16ern6hecjvsvga0yb6kgdhbtvynms28s8mpxdsf1hf3r00',
	),
]


def test_round_trip():
	for hexs, b32 in CASES:
		raw = bytes.fromhex(hexs)
		assert encode(raw) == b32
		assert decode(b32) == raw


def test_empty():
	assert encode(b'') == ''
	assert decode('') == b''


def test_case_insensitive_and_aliases():
	raw = bytes.fromhex('47b2d8f260c2d48116044bc43fe3de0f')
	canonical = '8ysdhwk0rba825g49f23zryy1w'
	assert decode(canonical.upper()) == raw
	assert decode('8ysd-hwk0-rba8-25g4-9f23-zryy-1w') == raw
	# crockford aliases: i/l -> 1, o -> 0
	assert decode('ooiill') == decode('001111')


def test_rejects_invalid():
	assert decode('u') is None  # not in alphabet, not an alias
	assert decode('01') is None  # non-zero trailing padding bits
