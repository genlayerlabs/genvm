"""
GenVM base32 encoder/decoder.

Uses `Crockford's Base32 <https://www.crockford.com/base32.html>`_: the alphabet
``0123456789abcdefghjkmnpqrstvwxyz`` (excludes ``i``, ``l``, ``o``, ``u``), no
padding, big-endian bit packing. Encoding is lowercase by default; decoding is
case-insensitive, treats ``i``/``l`` as ``1`` and ``o`` as ``0``, and ignores
``-``. Used to encode hashes (such as runner ids). Mirrors the Rust
``genlayer_sdk::gvm32`` implementation.
"""

__all__ = ('encode', 'decode')

import collections.abc

_ALPHABET = '0123456789abcdefghjkmnpqrstvwxyz'


def encode(data: collections.abc.Buffer) -> str:
	"""
	Encode a byte buffer to its Crockford Base32 string (lowercase, no padding).

	:param data: bytes to encode
	:returns: the base32-encoded string
	"""
	out: list[str] = []
	value = 0
	bits = 0
	for byte in bytes(data):
		value = (value << 8) | byte
		bits += 8
		while bits >= 5:
			bits -= 5
			out.append(_ALPHABET[(value >> bits) & 0x1F])
	if bits > 0:
		out.append(_ALPHABET[(value << (5 - bits)) & 0x1F])
	return ''.join(out)


def _decode_char(ch: str) -> int | None:
	c = ch.lower()
	if c == 'o':
		c = '0'
	elif c in ('i', 'l'):
		c = '1'
	idx = _ALPHABET.find(c)
	return idx if idx >= 0 else None


def decode(s: str) -> bytes | None:
	"""
	Decode a Crockford Base32 string back to bytes.

	Case-insensitive; ``i``/``l`` are read as ``1``, ``o`` as ``0`` and ``-`` is
	ignored.

	:param s: base32 string
	:returns: the decoded bytes, or ``None`` if ``s`` contains an invalid
		character or has non-zero trailing padding bits
	"""
	out = bytearray()
	value = 0
	bits = 0
	for ch in s:
		if ch == '-':
			continue
		digit = _decode_char(ch)
		if digit is None:
			return None
		value = (value << 5) | digit
		bits += 5
		if bits >= 8:
			bits -= 8
			out.append((value >> bits) & 0xFF)
	# any leftover bits are padding and must be zero
	if value & ((1 << bits) - 1):
		return None
	return bytes(out)
