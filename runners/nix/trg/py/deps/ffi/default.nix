{ pkgs
, lib
, wasmShell
, ...
}:
wasmShell.stdenv.mkDerivation {
	pname = "genvm-ffi";
	version = "3.4.6";

	outputHash = "sha256-ZN+6jmuivB37sWfwYhUi3A8/uqBxMpjfEU1DNd4COQA=";
	outputHashMode = "recursive";

	srcs = [
		(pkgs.fetchzip {
			url = "https://github.com/libffi/libffi/releases/download/v3.4.6/libffi-3.4.6.tar.gz";
			sha256 = "sha256-5kYA8yUGBeIA8eCRDM8CLWRsvKmNj5nWhl3+zl5RIhU=";
			name = "genvm-ffi-src";
		})
		(builtins.path { name = "stub_ffi.c"; path = ./stub_ffi.c; })
	];

	unpackPhase = ''
		for s in $srcs
		do
			echo "src === $s"
			if [[ "$s" == *.c ]]
			then
				cp "$s" ./"$(stripHash "$s")"
			else
				cp -r "$s"/* .
			fi
		done
	'';

	nativeBuildInputs = [wasmShell.sdk];

	configurePhase = ''
		export ${wasmShell.envStr}
		export CFLAGS="$CFLAGS -Iinclude -Iwasm32-unknown-wasip1 -Iwasm32-unknown-wasip1/include"
		echo "$CFLAGS" > .cflags
		./configure \
			"--prefix=$out" \
			--host=wasm32-wasip1
	'';

	buildPhase = builtins.readFile ./build.sh;

	installPhase = ''
		mkdir -p "$out/lib"
		mkdir -p "$out/include"

		make install-pkgconfigDATA
		make install-info
		make install-data

		cp libffi.a "$out/lib"

		rm -rf "$out/lib/pkgconfig/" || true
		rm -rf "$out/share/man/" || true
	'';

	dontFixup = true;
	dontPatchELF = true;
}
