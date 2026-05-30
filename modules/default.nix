{ pkgs
, root-src
, compile-rust
, get-root-subtree
, build-config
, patch-yaml-schema
, patch-manifest
, patch-llm-config
, patch-web-config
, patch-rpath
, host-system
, host-system-as-genvm
, compiled-libs
, ...
}@args:
let
	lib = pkgs.lib;

	make-for-target = target:
		let
			exe = compile-rust rec {
				inherit target;
				pname = "genvm-modules-bin";
				version = "0.1.0";

				profile = "release-with-debug";

				cargoLock.lockFile = ./implementation/Cargo.lock;

				src = get-root-subtree [
					"modules/implementation"
					"modules/interfaces"
					"executor/crates"
				];
				sourceRoot = "./source/modules/implementation";

				extraLibs = compiled-libs.${target};

				LUA_LIB_NAME = "lua";

				LSQLITE3_PREBUILT = "1";

				GENVM_PROFILE = build-config.executor-version;
			};
		in pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-manager-${target}";

			srcs = [
				exe
				./install
			] ++ compiled-libs.${target};


			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;

			nativeBuildInputs = [ pkgs.makeWrapper patch-yaml-schema patch-manifest patch-llm-config patch-web-config patch-rpath ];

			installPhase = ''
				mkdir -p $out/bin
				mkdir -p $out/lib
				cp ${exe} "$out/bin/genvm-modules"
				for src in $srcs; do
					if [[ "$src" != "${exe}" ]]
					then
						cp --no-preserve=ownership -r "$src/." "$out/."
						chmod -R u+w "$out"
					fi
				done
				patch-yaml-schema --tag ${build-config.executor-version} "$out"
				patch-llm-config --tag ${build-config.executor-version} "$out/config/genvm-module-llm.yaml"
				patch-web-config --tag ${build-config.executor-version} "$out/config/genvm-module-web.yaml"
				patch-manifest --tag ${build-config.executor-version} "$out/data/manifest.yaml"

				patch-rpath --codesign --search-dir "$out/lib" --rpath '$ORIGIN/../lib' "$out/bin/genvm-modules"
				find "$out/lib" -type f -name '*.so' -not -name 'libc.so' | while read lib; do
					patch-rpath --search-dir "$out/lib" --rpath '$ORIGIN' "$lib"
				done
			'';
		};
in {
	manager = make-for-target host-system-as-genvm;
	manager-amd64-linux = make-for-target "amd64-linux";
	manager-arm64-linux = make-for-target "arm64-linux";
	manager-arm64-macos = make-for-target "arm64-macos";
}
