{ pkgs
, lib
, ...
}:
let
	wasi-sdk-raw = (pkgs.fetchzip {
		name = "wasi-sdk-raw";
		url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz";
		hash = "sha256-/cyLxhFsfBBQxn4NrhLdbgHjU3YUjYhPnvquWJodcO8=";
	});
	wasi-sdk = pkgs.stdenvNoCC.mkDerivation {
		name = "wasi-sdk";
		version = "24.0";

		src = wasi-sdk-raw;

		buildInputs = [pkgs.libgcc pkgs.texinfo];

		nativeBuildInputs = [pkgs.autoPatchelfHook];

		dontConfigure = true;
		dontBuild = true;

		installPhase = ''
			mkdir -p "$out"
			cp -r * "$out/"
			autoPatchelf "$out"

			"$out/bin/clang" --version
		'';
	};
in rec {
	package = wasi-sdk;

	env = rec {
		CC = "${toString wasi-sdk}/bin/clang";
		CXX = "${toString wasi-sdk}/bin/clang++";
		CFLAGS = "-fdebug-prefix-map=${toString wasi-sdk}=/wasi-sdk -flto -Wno-builtin-macro-redefined -D__TIME__='\"00:42:42\"' -D__DATE__='\"Jan_24_2024\"' -O3 --sysroot=${toString wasi-sdk}/share/wasi-sysroot --target=wasm32-wasip1 -g -frandom-seed=4242 -no-canonical-prefixes";
		CXXFLAGS = CFLAGS;
		LD = "${toString wasi-sdk}/bin/wasm-ld";
	};

	env-str =
		builtins.concatStringsSep
			" "
			(builtins.map
				(name: "${name}='${builtins.replaceStrings [ "'" ] [ "'\"'\"'" ] env.${name}}'")
				(builtins.attrNames env))
	;
}
