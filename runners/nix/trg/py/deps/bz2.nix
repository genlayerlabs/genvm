{ pkgs
, lib
, wasmShell
, ...
}:
wasmShell.stdenv.mkDerivation {
	pname = "genvm-bz2";
	version = "1.0.8";

	outputHash = "sha256-wLBL3IDXXSTJX9W6JGR5Aw7nHPRAG/uT9aBEM/FKzfU=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		url = "https://sourceware.org/pub/bzip2/bzip2-1.0.8.tar.gz";
		sha256 = "Uvi4JZPPERK3gym4yoaeTEJwKXF5brBAEN7GgF+iF6g=";
		name = "genvm-bzip2-src";
	};

	nativeBuildInputs = [wasmShell.sdk];

	dontConfigure = true;

	buildPhase = ''
		make ${wasmShell.envStr} -j libbz2.a
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		mkdir -p "$out/include"

		cp libbz2.a "$out/lib"
		cp bzlib.h "$out/include"
	'';

	dontFixup = true;
	dontPatchELF = true;
}
