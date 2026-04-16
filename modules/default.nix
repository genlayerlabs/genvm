{ pkgs
, root-src
, compile-rust
, components
, get-root-subtree
, build-config
, patch-yaml-schema
, patch-manifest
, patch-llm-config
, patch-web-config
, ...
}:
let
	lib = pkgs.lib;
	make-for-target = target:
		let
			exe = compile-rust rec {
				inherit target;
				pname = "genvm-modules-bin";
				version = "0.1.0";

				profile = "release-with-debug";

				# Modules link dynamically against liblua.so, so disable
				# musl static CRT linking to allow dynamic library search.
				omitDefaultRustFlags = true;

				cargoLock.lockFile = ./implementation/Cargo.lock;

				src = get-root-subtree [
					"modules/implementation"
					"modules/interfaces"
					"executor/crates/common"
					"executor/crates/sdk-rs"
					"executor/crates/calldata"
				];
				sourceRoot = "./source/modules/implementation";

				extraLibs = [
					components.${target}.liblua
				] ++ (if target == "arm64-macos" then [ components.${target}.libiconv ] else [ components.${target}.libc ]);

				LUA_LIB_NAME = "lua";

				GENVM_PROFILE = build-config.executor-version;
			};
		in pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-modules-${target}";

			srcs = [
				exe
				./install
			];


			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;

			nativeBuildInputs = [ pkgs.makeWrapper patch-yaml-schema patch-manifest patch-llm-config patch-web-config ];

			installPhase = ''
				mkdir -p $out/bin
				cp ${exe} "$out/bin/genvm-modules"
				for src in $srcs; do
					if [[ "$src" != "${exe}" ]]
					then
						cp --no-preserve=ownership -r "$src/." "$out/."
					fi
				done

				chmod -R u+w "$out"
				patch-yaml-schema --tag ${build-config.executor-version} "$out"

				patch-manifest --tag ${build-config.executor-version} "$out/data/manifest.yaml"
				patch-llm-config --tag ${build-config.executor-version} "$out/config/genvm-module-llm.yaml"
				patch-web-config --tag ${build-config.executor-version} "$out/config/genvm-module-web.yaml"
			'';
		};
in {
	amd64-linux = {
		modules = make-for-target "amd64-linux";
	};
	arm64-linux = {
		modules = make-for-target "arm64-linux";
	};
	arm64-macos = {
		modules = make-for-target "arm64-macos";
	};
}
