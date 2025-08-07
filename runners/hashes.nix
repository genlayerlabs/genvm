let
	src = rec {
		__prefix = "";

		models = {
			__prefix = "models-";

			all-MiniLM-L6-v2 = {
				hash = "sha256-C3vqRgr76VlY0G+HaZeGMrco+ya77R9mNE5bLWXE0Ok=";
			};
		};

		pyLibs = {
			__prefix = "py-lib-";

			cloudpickle = {
				hash = "sha256-DT6YulshSrNfvWQvVLNZHfHvfFVlmDTy+SAu5Ww1k7Y=";
			};
			protobuf = {
				hash = "sha256-gbbFLhPUVsefodopQthwDPhwDmlDU06rZUPVMLeJOWY=";
			};

			word_piece_tokenizer = {
				hash = "sha256-GI/J7iTPhXqn7RfUhIfQk/p8Zwi31vGIft5bvPLcnlQ=";
			};

			genlayer-std = {
				hash = "sha256-e3iIRWICVVbk1DEkanSffdk/fTVRufM1V1I5LwCwMIc=";
			};

			genlayer-embeddings = {
				hash = "sha256-4aH3t6P7KZpO3v7UTPF7W2lIqKuMQAfoVCv5HlxP4Jo=";

				depends = [
					models.all-MiniLM-L6-v2
					pyLibs.word_piece_tokenizer
					pyLibs.protobuf
				];
			};
		};

		cpython = {
			hash = "sha256-QEdbmqYOjTOnNRCozWWt825mPSTOK8HbUhCgRUhbxug=";
			depends = [
				softfloat
			];
		};

		softfloat = {
			hash = "sha256-lkSLHic0pVxCyuVcarKj80FKSxYhYq6oY1+mnJryZZ0=";
		};

		wrappers = {
			__prefix = "";
			py-genlayer = {
				hash = "sha256-0K2PpnxgwsO5e7CcE2Q3Skr0drCXg/u8d1QVuW42TVE=";
				depends = [
					cpython
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = "sha256-ws9mTY8AhgzrsVSgtDBQjeFo4SFMW3vTbOSZqzeMfd0=";
				depends = [
					cpython
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
		};
	};

	genVMAllowTest = import ./dbg.nix;

	hashHasSpecial = hsh: val:
		if val.hash == hsh
		then true
		else hashHasSpecialDeps hsh val;

	hashHasSpecialDeps = hsh: val:
		builtins.any (hashHasSpecial hsh) (if builtins.hasAttr "depends" val then val.depends else []);

	deduceHash = val:
		if hashHasSpecial "test" val
		then (if genVMAllowTest then "test" else "error")
		else if val.hash == null
		then null
		else if hashHasSpecial null val
		then "error"
		else val.hash;

	fakeHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

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
	transform "" "" src
