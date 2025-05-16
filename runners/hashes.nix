let
	src = rec {
		__prefix = "";

		pyLibs = {
			__prefix = "py-lib-";

			cloudpickle = {
				hash = "sha256-LJm85ypTY7TlSih1pzKu7IsYHZUUfeaq76zb4gN9JBs=";
			};
			protobuf = {
				hash = "sha256-Sp879LjcoRMhX764CBqydwBfpcxoJCDP2nS6vVqhsmA=";
			};
			tiny_onnx_reader = {
				hash = "sha256-UYz1TbuI+DJbcjaLIEZ7CCop/nb59GJzXSSR5xnzImE=";
			};
			word_piece_tokenizer = {
				hash = "sha256-cHaMUVyCB8GgpEILVZqrdniyg8waU2naNlAkR2oUp/A=";
			};
			genlayermodelwrappers = {
				hash = "sha256-X/PGJc8fIvYC+KXCs35VVasD+AMNeQCRy/FnLQsEU/Y=";
			};
			genlayer-std = {
				hash = "test";
				depends = [
					cpython
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
				hash = null;
				depends = [
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = null;
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
		then assert (genVMAllowTest || builtins.throw "test hash not allowed"); "test"
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
