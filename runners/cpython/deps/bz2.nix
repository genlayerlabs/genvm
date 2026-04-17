{ pkgs
, lib
, stdenvNoCC
, runnersLib
, deps
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-bz2";
	version = "1.0.8";

	outputHash = "sha256-NQdALM+p13cAlPKMBY8E2vH6/qEZJ6R+ktLgDZuEXxo=";
	outputHashMode = "recursive";

	src = deps."genvm-bz2-src";

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
