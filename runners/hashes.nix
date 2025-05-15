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
				hash = "sha256-tZUcCZ9to8S9pnS8DwGdZmNOvZMFBzxZop7t4nmF8wc=";
				depends = [
					cpython
				];
			};
		};

		cpython = {
			hash = "sha256-FpbbAgWDgf5HUzfSMDrcuAIDciU63szzeMPPV28Svi0=";
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
				hash = "sha256-j+umxC6V+S2R0iyaNiHCgbeuymapa+LSyi0+iGSX6Fc=";
				depends = [
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
			py-genlayer-multi = {
				hash = "sha256-K64Wxx1/vASQR/0bBvCxEqLxcyXR5fMUSFEWzIB7pbM=";
				depends = [
					pyLibs.cloudpickle
					pyLibs.genlayer-std
				];
			};
		};
	};

	hasHash = val:
		if val.hash == null
		then false
		else builtins.all hasHash (if builtins.hasAttr "depends" val then val.depends else []);

	depsAreUpToDate = val: builtins.all hasHash (if builtins.hasAttr "depends" val then val.depends else []);

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
				hash64 =
					if val.hash != null
					then assert depsAreUpToDate val || builtins.throw "${pref}${name} set hash to null (null dependency)"; val.hash
					else fakeHash;
				hash32 = builtins.convertHash { hash = hash64; toHashFormat = "nix32"; };
			in rec {
				id = pref + name;

				hash = hash64;

				uid = "${id}:${hash32}";

				excludeFromBuild = val.hash == null && !(depsAreUpToDate val);
			}
	);
in
	transform "" "" src
