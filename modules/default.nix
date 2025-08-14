{ pkgs
, root-src
, compile-rust
, components
, get-root-subtree
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

				cargoLock.lockFile = ./implementation/Cargo.lock;

				src = get-root-subtree [
					"modules/implementation"
					"modules/interfaces"
					"executor/common"
				];
				sourceRoot = "./source/modules/implementation";

				extraLibs = [
					components.${target}.libc
					components.${target}.liblua
				];

				LUA_LIB_NAME = "lua-${components.${target}.liblua.version}";
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

			nativeBuildInputs = [ pkgs.makeWrapper ];

			installPhase = ''
				mkdir -p $out/bin
				cp ${exe} "$out/bin/genvm-modules"
				for src in $srcs; do
					if [[ "$src" != "${exe}" ]]
					then
						cp -r "$src/." "$out/."
					fi
				done
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
