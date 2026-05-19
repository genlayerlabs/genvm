{ pkgs
, root-src
, compile-rust
, get-root-subtree
, build-config
, patch-yaml-schema
, patch-rpath
, host-system
, host-system-as-genvm
, compiled-libs
, ...
}@args:
let
	manifests-data = import ../runners/support/views/release.nix args;

	lib = pkgs.lib;
	make-for-target = target:
		let
			exe = compile-rust rec {
				inherit target;

				pname = "genvm-executor";
				version = build-config.executor-version;

				cargoLock.lockFile = ./Cargo.lock;

				src = get-root-subtree [
					"executor/src"
					"modules/interfaces"
					"executor/crates"
					"executor/third-party"
					"executor/Cargo.toml"
					"executor/Cargo.lock"
					"doc/schemas"
				];
				sourceRoot = "./source/executor";

				extraLibs = compiled-libs.${target};

				GENVM_PROFILE = build-config.executor-version;
			};
		in pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-executor-${target}";

			srcs = [
				exe
				./install
			];

			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;

			nativeBuildInputs = [
				pkgs.makeWrapper
				patch-yaml-schema
				patch-rpath
			];

			installPhase = ''
				mkdir -p $out/executor/${build-config.executor-version}/bin

				mkdir -p $out/executor/${build-config.executor-version}/data
				cp "${manifests-data.latest}" "$out/executor/${build-config.executor-version}/data/latest.json"
				cp "${manifests-data.all}" "$out/executor/${build-config.executor-version}/data/all.json"

				cp "${exe}" "$out/executor/${build-config.executor-version}/bin/genvm"
				for src in $srcs; do
					if [[ "$src" != "${exe}" ]]
					then
						cp --no-preserve=ownership -r "$src/." "$out/executor/${build-config.executor-version}/."
					fi
				done

				chmod -R u+w "$out"
				patch-yaml-schema --tag ${build-config.executor-version} "$out"

				patch-rpath --codesign \
					--rpath '$ORIGIN/../lib' \
					--rpath '$ORIGIN/../../../lib' \
					"$out/executor/${build-config.executor-version}/bin/genvm"
			'';
		};
in {
	executor = make-for-target host-system-as-genvm;

	executor-amd64-linux = make-for-target "amd64-linux";
	executor-arm64-linux = make-for-target "arm64-linux";
	executor-arm64-macos = make-for-target "arm64-macos";
}
