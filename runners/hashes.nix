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
				hash = "sha256-LJm85ypTY7TlSih1pzKu7IsYHZUUfeaq76zb4gN9JBs=";
			};
			protobuf = {
				hash = "sha256-Sp879LjcoRMhX764CBqydwBfpcxoJCDP2nS6vVqhsmA=";
			};

			word_piece_tokenizer = {
				hash = "sha256-cHaMUVyCB8GgpEILVZqrdniyg8waU2naNlAkR2oUp/A=";
			};

			genlayer-std = {
				hash = "sha256-U2J42ZeXgD4XExr0OeRExz0TkWUBZIx4y9gXyoqwFWo=";
				depends = [
					cpython
				];
			};

			genlayer-embeddings = {
				hash = "sha256-MzlkqoMA3aQzqfvu9BYkGsUdT3MXlttQUJvXeU7xuAM=";

				depends = [
					models.all-MiniLM-L6-v2
					pyLibs.word_piece_tokenizer
					pyLibs.protobuf
				];
			};
		};

		cpython = {
			hash = "sha256-e6ZqT1G5w7wNNiKycS35xHCP/wn4zbW11FOtfZlSxlg=";
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
				hash = "sha256-HkkgBgbPTKmyjZ1Dfj1V+OIswoJLRbD4+JKnCzPKgY4=";
				depends = [
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = "sha256-jgmwwAnQnLeWuttPEe3Vz1PzgdylH5AN8n2s6u70FMs=";
				depends = [
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

				excludeFromBuild = deducedHash == null && (hashHasSpecialDeps null val);
			}
	);
in
	transform "" "" src
