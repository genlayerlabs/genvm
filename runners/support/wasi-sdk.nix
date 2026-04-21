{ pkgs
, lib
, ...
}:
let
	# wasi-sdk ships binaries per host platform — linux x86_64/arm64 and
	# macOS arm64/x86_64. The WASM output is platform-agnostic, but the
	# toolchain itself is native to the build host.
	tarballs = {
		"x86_64-linux" = {
			url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz";
			hash = "sha256-/cyLxhFsfBBQxn4NrhLdbgHjU3YUjYhPnvquWJodcO8=";
		};
		"aarch64-linux" = {
			url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-arm64-linux.tar.gz";
			# Populate on first arm64-linux build: nix will report the correct
			# sha256 and swap this placeholder in.
			hash = lib.fakeHash;
		};
		"aarch64-darwin" = {
			url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-arm64-macos.tar.gz";
			hash = lib.fakeHash;
		};
		"x86_64-darwin" = {
			url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-macos.tar.gz";
			hash = lib.fakeHash;
		};
	};
	system = pkgs.stdenv.hostPlatform.system;
	artifact = tarballs.${system} or
		(throw "wasi-sdk: no packaged tarball for ${system}");

	wasi-sdk-raw = pkgs.fetchzip {
		name = "wasi-sdk-raw";
		url = artifact.url;
		hash = artifact.hash;
	};
	wasi-sdk = pkgs.stdenvNoCC.mkDerivation {
		name = "wasi-sdk";
		version = "24.0";

		src = wasi-sdk-raw;

		# autoPatchelf is Linux-only; on darwin the toolchain ships as
		# Mach-O binaries that don't need ELF patching.
		buildInputs =
			lib.optionals pkgs.stdenv.isLinux [ pkgs.libgcc pkgs.texinfo ];

		nativeBuildInputs =
			lib.optional pkgs.stdenv.isLinux pkgs.autoPatchelfHook;

		dontConfigure = true;
		dontBuild = true;

		installPhase = ''
			mkdir -p "$out"
			cp -r * "$out/"
			${lib.optionalString pkgs.stdenv.isLinux ''
				autoPatchelf "$out"
			''}

			"$out/bin/clang" --version
		'';
	};
in rec {
	package = wasi-sdk;

	env = rec {
		CC = "${toString wasi-sdk}/bin/clang";
		CXX = "${toString wasi-sdk}/bin/clang++";
		CFLAGS = "-fdebug-prefix-map=${toString wasi-sdk}=/wasi-sdk -flto -Wno-builtin-macro-redefined -D__TIME__='\"00:42:42\"' -D__DATE__='\"Jan_24_2024\"' -O2 --sysroot=${toString wasi-sdk}/share/wasi-sysroot --target=wasm32-wasip1 -g -frandom-seed=4242 -no-canonical-prefixes";
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
