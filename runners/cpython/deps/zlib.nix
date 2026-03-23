{ pkgs
, lib
, stdenvNoCC
, runnersLib
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-zlib";
	version = "1.3.1";

	outputHash = "sha256-+v15QTVc9fl1+3Bn9jVEadEaZw5CjaPa0fmJO/eTGZ4=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		urls = [
			"https://www.zlib.net/zlib-1.3.1.tar.gz"
			"https://storage.googleapis.com/genvm-artifacts/zlib-1.3.1.tar.gz"
		];
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
