{ pkgs
, lib
, stdenvNoCC
, runnersLib
, deps
, ...
}:
stdenvNoCC.mkDerivation {
	pname = "genvm-xz";
	version = "5.6.2";

	outputHash = "sha256-gGil/LMGNqbA+jQ/kya1AiEp8/ihgI99JshKKlz5vb0=";
	outputHashMode = "recursive";

	src = deps."genvm-xz-src";

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
