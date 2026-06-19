let
	dev-mode = import ./dev-mode.nix;

	src = rec {
		__prefix = "";

		models = {
			__prefix = "models-";

			all-MiniLM-L6-v2 = {
				hash = "test";
			};
		};

		pyLibs = {
			__prefix = "py-lib-";

			cloudpickle = {
				hash = "test";
			};
			protobuf = {
				hash = "test";
			};

			word_piece_tokenizer = {
				hash = "test";
			};

			genlayer-std = {
				hash = "test";
			};

			genlayer-embeddings = {
				hash = "test";

				depends = [
					models.all-MiniLM-L6-v2
					pyLibs.word_piece_tokenizer
					pyLibs.protobuf
				];
			};
		};

		cpython = {
			hash = "test";
			depends = [
				softfloat
			];
		};

		softfloat = {
			hash = "test";
		};

		wrappers = {
			__prefix = "";
			py-genlayer = {
				hash = "test";
				depends = [
					cpython
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = "test";
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
				hash32 = if deducedHash == "test" then "test" else builtins.convertHash { hash = hashSRI; toHashFormat = "nix32"; };
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
