{ pkgs
, lib
, stdenvNoCC
, runnersLib
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-bz2";
	version = "1.0.8";

	outputHash = "sha256-upfoJkaRbotTurv2JQgDotzp76ZHUCllxEb07MeiCO0=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		url = "https://sourceware.org/pub/bzip2/bzip2-1.0.8.tar.gz";
		sha256 = "Uvi4JZPPERK3gym4yoaeTEJwKXF5brBAEN7GgF+iF6g=";
		name = "genvm-bz2-src";
	};

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
