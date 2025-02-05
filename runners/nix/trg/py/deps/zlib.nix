{ pkgs
, lib
, wasmShell
, ...
}:
wasmShell.stdenv.mkDerivation {
	pname = "genvm-zlib";
	version = "1.3.1";

	outputHash = "sha256-qdvdo6vihKpxlOGsaeWUn9QBbBvatB1KJpkp4osdgC8=";
	outputHashMode = "recursive";

	src = pkgs.fetchzip {
		url = "https://www.zlib.net/zlib-1.3.1.tar.gz";
		sha256 = "acY8yFzIRYbrZ2CGODoxLnZuppsP6KZy19I9Yy77pfc=";
		name = "genvm-zlib-src";
	};

	nativeBuildInputs = [wasmShell.sdk];

	configurePhase = ''
		export ${wasmShell.envStr}
		./configure --prefix="$out" --static
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
