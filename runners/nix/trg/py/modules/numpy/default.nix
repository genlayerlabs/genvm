{ pkgs
, lib
, wasmShell
, pyShell
, rustShell
, ...
}@args:
let
	configurator = (import ../../. args).configurator;

	pyHeaders = wasmShell.stdenv.mkDerivation (configurator // {
		pname = "genvm-cpython-headers";

		outputHash = "sha256-/ukpaDfXbCOXpUWhBR1blxuhol8zdTQj9mYlx9Iu+P4=";
		outputHashMode = "recursive";

		nativeBuildInputs = configurator.nativeBuildInputs;

		buildPhase = ''
			make -j inclinstall
		'';

		installPhase = ''
			mkdir -p "$out/include"
			cp -r /build/out/include/python3.13/* "$out/include"
		'';
	});
in
wasmShell.stdenv.mkDerivation {
	name = "genvm-cpython-numpy";

	buildInputs = [
	];

	nativeBuildInputs = [
		pyShell.py
		pyShell.cython
		pkgs.perl
		pkgs.ninja
	];

	srcs = [
		(builtins.fetchGit {
			url = "https://github.com/numpy/numpy.git";
			rev = "3fcac502eba9523718f8e2e3a4aaf83665165dfe";
			name = "genvm-cpython-numpy-src";
			submodules = true;
		})
		./deps
		wasmShell.sdk
		pyHeaders
	];

	outputHash = "sha256-bVqHI5TW5SqULCrg4gPfnVDexKga4QBtz8BWLQvACw4=";
	outputHashMode = "recursive";

	sourceRoot = "genvm-cpython-numpy-src";

	patches = [
		./patches/1
	];

	configurePhase = ''
		ls -l /build/genvm-cpython-headers-3.13/include
		chmod -R +w /build/deps/
		FROM='#!/usr/bin/env python3' TO='#!${pyShell.py.outPath}/bin/python3' perl -pe 's/$ENV{FROM}/$ENV{TO}/g' -i /build/deps/stub-clang.py

		echo 'c_args = '"'"'${wasmShell.env.CFLAGS} -D__EMSCRIPTEN__ -I/build/genvm-cpython-headers-3.13/include'"'" >> /build/deps/cross-file.txt
		echo 'cpp_args = '"'"'${wasmShell.env.CXXFLAGS} -D__EMSCRIPTEN__ -I/build/genvm-cpython-headers-3.13/include -fno-rtti'"'" >> /build/deps/cross-file.txt

		mkdir -p /build/path
		ln -s /build/wasi-sdk/bin/ar /build/path/ar
		export PATH="/build/path:$PATH"

		python3 vendored-meson/meson/meson.py setup --cross-file /build/deps/cross-file.txt build-wasm --prefix /build/out
	'';

	buildPhase = ''
		pushd build-wasm
		python3 ../vendored-meson/meson/meson.py install --tags runtime,python-runtime
		popd

		find /build/out -type f -and -name '__config__.py' | xargs perl -i -pe 's/"args": r".*",/"args": r"",/'
		find /build/out -type f -and -name '__config__.py' | xargs perl -i -pe 's/\/build\/|(\/nix\/store[^-]*)/\/np\//g'
		find /build/out -type f -and -name '*.pyc' -delete

		AR_SCRIPT="CREATE /build/out/numpy.a"
		for f in $(find /build/out -type f -and -name '*.so' | sort)
		do
			AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDLIB $f"
		done

		/build/wasi-sdk/bin/clang ${wasmShell.env.CFLAGS} -o cxx-abi-stub.o -c /build/deps/cxx-abi-stub.c
		AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDMOD cxx-abi-stub.o"

		AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
		AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

		echo "$AR_SCRIPT" | ar -M

		find /build/out/lib -type f -name '*.so' -or -name '*.h' -or -name '*.c' -delete
		find /build/out -type d -empty -delete
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		mkdir -p "$out/py"
		cp /build/out/numpy.a "$out/lib/libnumpy.a"

		cp -r /build/out/lib/python3.13/site-packages/numpy/ "$out/py"
		cp -r /build/deps/override/* "$out/py/numpy/"
		cp /build/deps/Setup.local "$out/"
	'';
}
