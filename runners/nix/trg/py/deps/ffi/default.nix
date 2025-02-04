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
		chmod -R +w .
	'';

	nativeBuildInputs = [wasmShell.sdk];

	configurePhase = ''
		export ${wasmShell.envStr}
		export CFLAGS="$CFLAGS -Iinclude -Iwasm32-unknown-wasip1 -Iwasm32-unknown-wasip1/include"
		./configure \
			"--prefix=$out" \
			--host=wasm32-wasip1
	'';

	buildPhase = ''
		AR_SCRIPT="CREATE libffi.a"

		for i in stub_ffi.c src/closures.c src/prep_cif.c src/tramp.c src/debug.c src/raw_api.c src/types.c
		do
			FNAME="$(basename "$i")"
			clang ${wasmShell.env.CFLAGS} \
				-o "$i.o" \
				-fPIC \
				-Iinclude -Iwasm32-unknown-wasip1 -Iwasm32-unknown-wasip1/include \
				-c "$i"
			AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDMOD $i.o"
		done

		AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
		AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

		echo "$AR_SCRIPT" | ar -M
	'';

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
