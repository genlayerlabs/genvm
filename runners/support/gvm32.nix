# GenVM base32 (Crockford's Base32, lowercase) encoder.
#
# Mirrors executor/crates/sdk-rs/src/gvm32.rs and
# runners/genlayer-py-std/src/genlayer/gvm32.py. Used to derive runner content
# hash ids. Implemented per output symbol (not by accumulating the whole value)
# so it never overflows Nix's 64-bit integers, even for long hashes.
{ lib }:
let
	alphabet = lib.stringToCharacters "0123456789abcdefghjkmnpqrstvwxyz";

	# Nix `/` is integer division for ints; there is no `%` operator.
	mod = a: b: a - b * (a / b);
	pow2 = n: builtins.foldl' (acc: _: acc * 2) 1 (lib.range 1 n);

	hexValues = {
		"0" = 0; "1" = 1; "2" = 2; "3" = 3; "4" = 4; "5" = 5; "6" = 6; "7" = 7;
		"8" = 8; "9" = 9; "a" = 10; "b" = 11; "c" = 12; "d" = 13; "e" = 14;
		"f" = 15; "A" = 10; "B" = 11; "C" = 12; "D" = 13; "E" = 14; "F" = 15;
	};

	# hex string -> list of byte integers
	hexToBytes = hex:
		let
			chars = lib.stringToCharacters hex;
			charsLen = builtins.length chars;
			# integer division truncates, so reject odd lengths instead of
			# silently dropping the final nibble (which would corrupt the hash).
			n =
				if charsLen / 2 * 2 != charsLen
				then throw "gvm32.hexToBytes: hex string length must be even"
				else charsLen / 2;
		in
		builtins.genList
			(i: hexValues.${builtins.elemAt chars (i * 2)} * 16
				+ hexValues.${builtins.elemAt chars (i * 2 + 1)})
			n;

	# list of byte integers -> Crockford Base32 string
	encode = bytes:
		let
			len = builtins.length bytes;
			byteAt = i: if i < len then builtins.elemAt bytes i else 0;
			nsym = (len * 8 + 4) / 5; # ceil(len * 8 / 5)
			symAt = k:
				let
					b = k * 5;
					i = b / 8;
					j = mod b 8;
					# 16-bit big-endian window over bytes[i], bytes[i+1]
					window = byteAt i * 256 + byteAt (i + 1);
					v = mod (window / (pow2 (11 - j))) 32;
				in
				builtins.elemAt alphabet v;
		in
		lib.concatStrings (builtins.genList symAt nsym);

	encodeHex = hex: encode (hexToBytes hex);
in
{
	inherit encode encodeHex;
}
