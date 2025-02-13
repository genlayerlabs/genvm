{ pkgs
, lib
, ...
}:
let
	wasiSDKRaw = (pkgs.fetchzip {
		name = "wasi-sdk-raw";
		url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz";
		sha256 = "/cyLxhFsfBBQxn4NrhLdbgHjU3YUjYhPnvquWJodcO8=";
	});
	wasiSDK = pkgs.stdenvNoCC.mkDerivation {
		name = "wasi-sdk";
		version = "24.0";

		src = wasiSDKRaw;

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
	wasiSDKPath = wasiSDK.outPath;
in rec {
	stdenv = pkgs.stdenvNoCC;

	sdk = wasiSDK;

	env = rec {
		WASI_ROOT = wasiSDKPath;
		CC = "${wasiSDKPath}/bin/clang";
		CXX = "${wasiSDKPath}/bin/clang++";
		CFLAGS = "-Wno-builtin-macro-redefined -D__TIME__='\"00:42:42\"' -D__DATE__='\"Jan_24_2024\"' -O3 --sysroot=${wasiSDKPath}/share/wasi-sysroot --target=wasm32-wasip1 -fPIC -g0 -frandom-seed=4242 -no-canonical-prefixes";
		CXXFLAGS = CFLAGS;
		LD = "${wasiSDKPath}/bin/wasm-ld";
	};

	path = "${wasiSDKPath}/bin/";

	envStr = lib.attrsets.foldlAttrs
		(acc: name: val: "${acc} ${name}=${lib.escapeShellArg val}")
		""
		env;
}
