{ pkgs
, root-src
, compile-rust
, components
, get-root-subtree
, build-config
, ...
}:
let
	lib = pkgs.lib;
	make-for-target = target:
		let
			exe = compile-rust rec {
				inherit target;

				pname = "genvm-bin";
				version = build-config.executor-version;

				cargoLock.lockFile = ./Cargo.lock;

				src = get-root-subtree [
					"executor/src"
					"modules/interfaces"
					"executor/common"
					"executor/third-party"
					"executor/Cargo.toml"
					"executor/Cargo.lock"
					"doc/schemas"
				];
				sourceRoot = "./source/executor";

				extraLibs = if target == "arm64-macos" then [ components.${target}.libiconv ] else [ components.${target}.libc ];

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

			nativeBuildInputs = [ pkgs.makeWrapper ];

			installPhase = ''
				mkdir -p $out/executor/${build-config.executor-version}/bin
				cp ${exe} "$out/executor/${build-config.executor-version}/bin/genvm"
				for src in $srcs; do
					if [[ "$src" != "${exe}" ]]
					then
						cp -r "$src/." "$out/executor/${build-config.executor-version}/."
					fi
				done
			'';
		};
in {
	amd64-linux = {
		executor = make-for-target "amd64-linux";
	};
	arm64-linux = {
		executor = make-for-target "arm64-linux";
	};
	arm64-macos = {
		executor = make-for-target "arm64-macos";
	};
}
