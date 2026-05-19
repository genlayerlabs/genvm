{ pkgs
, deps
, lib
, ...
}:
let
	binaryen-raw = deps."binaryen-raw-128";
	wasi-sdk-raw = deps."wasi-sdk-raw-31.0";
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
