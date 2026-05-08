{ pkgs
, deps
, lib
, stdenvNoCC
, runnersLib
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-bz2";
	version = "1.0.8";

	outputHash = "sha256-pfCzehsxNUEfGUHjFQ+r2FFad3To3ia5HBEWrcL0pdw=";
	outputHashMode = "recursive";

	src = deps."genvm-bz2-src-1.0.8";

	nativeBuildInputs = [ runnersLib.wasi-sdk.package ];

	dontConfigure = true;

	buildPhase = ''
		echo ${runnersLib.wasi-sdk.env-str}
		${runnersLib.wasi-sdk.env-str} make ${runnersLib.wasi-sdk.env-str} -j libbz2.a
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
