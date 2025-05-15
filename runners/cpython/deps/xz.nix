{ pkgs
, lib
, stdenvNoCC
, runnersLib
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-xz";
	version = "5.6.2";

	outputHash = "sha256-nuCqZXQrSyWfCyTgUGM/2k59t42ue2bOBMTP07gOaZM=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		url = "https://github.com/tukaani-project/xz/releases/download/v5.6.2/xz-5.6.2.tar.gz";
		sha256 = "sha256-3ELGir/E3YS9qWEYQ8SGFrU0md71/pl2QOyUIiH55BQ=";
		name = "genvm-xz-src";
	};

	nativeBuildInputs = [runnersLib.wasi-sdk.package];

	configurePhase = ''
		${runnersLib.wasi-sdk.env-str} ./configure \
			"--prefix=$out" \
			--host=wasm32-wasip1 \
			--enable-threads=no --enable-small --enable-decoders=lzma1,lzma2 \
			--disable-scripts --disable-doc
	'';

	buildPhase = ''
		make -C src/liblzma/ -j
	'';

	installPhase = ''
		make -C src/liblzma/ install
		rm -rf "$out/lib/pkgconfig/" || true
		rm "$out/lib/liblzma.la" || true
	'';

	dontPatchELF = true;
}
