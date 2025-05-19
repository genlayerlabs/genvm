{ stdenvNoCC
, pkgs
, runnersLib
, pythonObjs
, lib
, ...
}:
let
	extraObj = stdenvNoCC.mkDerivation {
		name = "genvm-cpython-mod-numpy-objs";

		outputHashMode = "recursive";
		outputHash = "sha256-mPO/PIDzqGr3sLTYs+SpT1YSYtIm3PKYJBhGJmbeUys=";

		srcs = [
			./deps
			(builtins.fetchGit {
				url = "https://github.com/numpy/numpy.git";
				rev = "3fcac502eba9523718f8e2e3a4aaf83665165dfe";
				name = "genvm-cpython-numpy-src";
				submodules = true;
				shallow = true;
			})
			pythonObjs
			runnersLib.wasi-sdk.package
		];

		sourceRoot = "genvm-cpython-numpy-src";

		patches = [
			./patches/1
		];

		nativeBuildInputs = [
			runnersLib.buildPy
			pkgs.perl
			runnersLib.wasi-sdk.package

			pkgs.python313Packages.cython
			pkgs.ninja
		];

		configurePhase = ''
			chmod -R +w /build/deps/
			FROM='#!/usr/bin/env python3' TO='#!${runnersLib.buildPy}/bin/python3' perl -pe 's/$ENV{FROM}/$ENV{TO}/g' -i /build/deps/stub-clang.py

			echo 'c_args = '"'"'${runnersLib.wasi-sdk.env.CFLAGS} -D__EMSCRIPTEN__ -I/build/${pythonObjs.name}/include/python3.13/'"'" >> /build/deps/cross-file.txt
			echo 'cpp_args = '"'"'${runnersLib.wasi-sdk.env.CXXFLAGS} -D__EMSCRIPTEN__ -I/build/${pythonObjs.name}/include/python3.13/ -fno-rtti'"'" >> /build/deps/cross-file.txt

			mkdir -p /build/path
			ln -s ${runnersLib.wasi-sdk.package}/bin/ar /build/path/ar
			export PATH="/build/path:$PATH"

			/build/deps/stub-clang.py --version

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

			mkdir -p obj

			for f in $(find /build/out -type f -and -name '*.so')
			do
				cp "$f" "obj/$(basename "$f").a"
			done

			/build/wasi-sdk/bin/clang ${runnersLib.wasi-sdk.env.CFLAGS} -o obj/cxx-abi-stub.o -c /build/deps/cxx-abi-stub.c

			find /build/out/lib -type f -name '*.so' -or -name '*.h' -or -name '*.c' -delete
			find /build/out -type d -empty -delete
		'';

		installPhase = ''
			mkdir -p "$out/obj"
			mkdir -p "$out/py/libs"

			cp -r obj/. "$out/obj/."

			cp -r /build/out/lib/python3.13/site-packages/numpy/ "$out/py/libs/"
			cp -r /build/deps/override/. "$out/py/libs/numpy/"
		'';
	};
in {
	extraObjs = [extraObj];
	runners = [];
	setupLines = [
		"_multiarray_umath"
		"_umath_linalg"
		"lapack_lite"
		"_pocketfft_umath"
		"_bounded_integers"
		"_sfc64"
		"_pcg64"
		"bit_generator"
		"_common"
		"_generator"
		"mtrand"
		"_mt19937"
	];
}
