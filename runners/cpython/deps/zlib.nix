{ pkgs
, lib
, stdenvNoCC
, runnersLib
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-zlib";
	version = "1.3.1";

	outputHash = "sha256-7qQEXxba8MaDA/mKTa8kdiGkf30beoBtDLeNDBeCRjA=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		url = "https://www.zlib.net/zlib-1.3.1.tar.gz";
		sha256 = "acY8yFzIRYbrZ2CGODoxLnZuppsP6KZy19I9Yy77pfc=";
		name = "genvm-zlib-src";
	};

	nativeBuildInputs = [runnersLib.wasi-sdk.package];

	configurePhase = ''
		${runnersLib.wasi-sdk.env-str} ./configure --prefix="$out" --static
	'';

	buildPhase = ''
		make -j
	'';

	installPhase = ''
		make install
		rm -rf "$out/lib/pkgconfig/" || true
		rm -rf "$out/share/man" || true
	'';

	dontPatchELF = true;
}
