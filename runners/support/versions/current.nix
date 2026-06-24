let
	dev-mode = import ./dev-mode.nix;

	# gvm32 (Crockford Base32) — the encoding the executor uses for runner hash
	# ids. The embedded `uid` (consumed by `Depends`) MUST use this, matching the
	# on-disk runner paths and `hashToIDHash` in ../default.nix.
	gvm32 = import ../gvm32.nix;

	src = rec {
		__prefix = "";

		models = {
			__prefix = "models-";

			all-MiniLM-L6-v2 = {
				hash = "sha256-OlZQmCyKA+ct9hV4G12vSRUYUYB2S1CdDjIG2ueaOjQ=";
			};
		};

		pyLibs = {
			__prefix = "py-lib-";

			cloudpickle = {
				hash = "sha256-X/SQxZDU/Ep9D5ptzccqMiewgUh6mtt3vMyA+ZnqrHA=";
			};
			protobuf = {
				hash = "sha256-i0mGEqZ5XvoYCK5EZLZe48BltX8UYDxsiBQ8wJDratE=";
			};

			word_piece_tokenizer = {
				hash = "sha256-3T57N4bs1eSBaudAg0Dwi87d7kbu+anEYlGFH4Q3ujA=";
			};

			genlayer-std = {
				hash = "sha256-/pdXFMD6CtosS0LH2fJbZkJg5E3sMFnpWrC3yXVONgQ=";
			};

			genlayer-embeddings = {
				hash = "sha256-Lw2tC5IdLPhw4M+Zfp2igB9NUI9m3PgNyUmb9GMSQWQ=";

				depends = [
					models.all-MiniLM-L6-v2
					pyLibs.word_piece_tokenizer
					pyLibs.protobuf
				];
			};
		};

		cpython = {
			hash = "sha256-LqsYHYqiAYnwDoXMpHOQHqvQCa+Dh9562J5ClfEggxI=";
			depends = [
				softfloat
			];
		};

		softfloat = {
			hash = "sha256-aR7i/mGXm+x7ofoL7iJ1zUHsVyqkAsZG0mQ/Lj1+l4c=";
		};

		wrappers = {
			__prefix = "";
			py-genlayer = {
				hash = "sha256-u9lPFVpXuye3z8SsOfjIwRzAKfRAPy4tmTUnA+um6K0=";
				depends = [
					cpython
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = "sha256-woB2R5aziFLNpKCjEquWAsorIPH4/EpaRdVVYJYSy3w=";
				depends = [
					cpython
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
		};
	};

	hashHasSpecial = hsh: val:
		if val.hash == hsh
		then true
		else hashHasSpecialDeps hsh val;

	hashHasSpecialDeps = hsh: val:
		builtins.any (hashHasSpecial hsh) (if builtins.hasAttr "depends" val then val.depends else []);

	deduceHash = val:
		if hashHasSpecial null val
		then null
		else if hashHasSpecial "test" val
		then "test"
		else val.hash;

	fakeHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

	checkHashes =  (pref: name: val:
		if builtins.hasAttr "__prefix" val then
			builtins.foldl'
				(acc: item: acc + item)
				""
				(builtins.map
					(name: checkHashes (pref + val.__prefix) name val.${name})
					(builtins.filter
						(name: name != "__prefix")
						(builtins.attrNames val)))
		else
			if val.hash == null
			then ""
			else if val.hash == "test" then
				(if dev-mode then "" else "set ${pref+name} hash to 'null'\n")
			else if hashHasSpecialDeps null val then
				"set ${pref+name} hash to null\n"
			else if hashHasSpecialDeps null val then
				"set ${pref+name} hash to 'test'\n"
			else
				""
	);

	transform = (pref: name: val:
		if builtins.hasAttr "__prefix" val then
			builtins.listToAttrs
				(builtins.map
					(name: {
						inherit name;
						value = transform (pref + val.__prefix) name val.${name};
					})
					(builtins.filter
						(name: name != "__prefix")
						(builtins.attrNames val)))
		else
			let
				deducedHashBase = deduceHash val;
				deducedHash = if deducedHashBase == "error" then builtins.throw "set ${pref+name} hash to null" else deducedHashBase;
				hashSRI =
					if deducedHash == null
					then fakeHash
					else deducedHash;
				hash32 = if deducedHash == "test" then "test" else gvm32.encodeHex (builtins.convertHash { hash = hashSRI; toHashFormat = "base16"; });
			in rec {
				id = pref + name;

				hash = hashSRI;

				uid = "${id}:${hash32}";

				excludeFromBuild = hashHasSpecialDeps null val;
			}
	);
in
	builtins.seq (
		let errs = checkHashes "" "" src; in
		if errs != "" then builtins.throw errs else null
	) (transform "" "" src)
