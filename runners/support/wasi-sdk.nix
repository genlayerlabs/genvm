{ pkgs
, lib
, ...
}:
let
	binaryen-raw = (pkgs.fetchzip {
		name = "binaryen-raw";
		url = "https://github.com/WebAssembly/binaryen/releases/download/version_128/binaryen-version_128-x86_64-linux.tar.gz";
		hash = "sha256-0AKDcwjrOLiC5roAsTb9dZqqrxDs2+E5e+2usQwrQgA=";
	});
	wasi-sdk-raw = (pkgs.fetchzip {
		name = "wasi-sdk-raw";
		url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-31/wasi-sdk-31.0-x86_64-linux.tar.gz";
		hash = "sha256-ZSI7b1wc1D3hqx3IqT8dlFVKldUFjQUZ7u48Cn8wbnE=";
	});
	wasi-sdk = pkgs.stdenvNoCC.mkDerivation {
		name = "wasi-sdk";
		version = "31.0";

		src = wasi-sdk-raw;

		buildInputs = [pkgs.gcc-unwrapped.lib pkgs.libgcc pkgs.texinfo pkgs.libtinfo pkgs.ncurses];

		nativeBuildInputs = [pkgs.autoPatchelfHook binaryen-raw];

		dontConfigure = true;
		dontBuild = true;

		installPhase = ''
			mkdir -p "$out"
			cp -r * "$out/"
			cp -r ${binaryen-raw}/. "$out/"
			autoPatchelf "$out"

			"$out/bin/clang" --version
			"$out/bin/wasm-opt" --version
		'';
	};
in rec {
	package = wasi-sdk;

	env = rec {
		CC = "${toString wasi-sdk}/bin/clang";
		CXX = "${toString wasi-sdk}/bin/clang++";
		CFLAGS = "-ffile-prefix-map=${toString wasi-sdk}=/wasi-sdk -flto -Wno-builtin-macro-redefined -D__TIME__='\"00:42:42\"' -D__DATE__='\"Jan_24_2024\"' -O2 --sysroot=${toString wasi-sdk}/share/wasi-sysroot --target=wasm32-wasip1 -g -frandom-seed=4242 -no-canonical-prefixes -mbulk-memory -msign-ext -mmutable-globals -mmultivalue -mtail-call -msimd128";
		CXXFLAGS = CFLAGS;
		LD = "${toString wasi-sdk}/bin/wasm-ld";
		WAT2WASMFLAGS = "--enable-tail-call --enable-annotations";
		WASMOPTFLAGS = "--inlining-optimizing --enable-tail-call --enable-multivalue --enable-bulk-memory-opt --enable-bulk-memory --enable-simd --enable-sign-ext --enable-mutable-globals";
	};

	env-str =
		builtins.concatStringsSep
			" "
			(builtins.map
				(name: "${name}='${builtins.replaceStrings [ "'" ] [ "'\"'\"'" ] env.${name}}'")
				(builtins.attrNames env))
	;
}
