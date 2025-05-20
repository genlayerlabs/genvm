# so, here is the issue:
# 1. we need to have python headers for (some) modules
# 2. we need modules to statically link them into cpython
# basically, we can just do work twice: firstly configure without local modules to just get headers
# and then secondly with them
# is it possible to configure once and then only link cpython.wasm manually with extra .a?


#-lnumpy -lc++ -lc++abi -lc-printscan-long-double
#-Wl,--stack-first
#-z stack-size=8388608

{ pkgs
, lib
, runnersLib
, stdenvNoCC
, ...
}@args:
let
	python-source = builtins.fetchGit {
		name = "cpython-313-src";
		url = "https://github.com/python/cpython.git";
		rev = "067145177975eadd61a0c907d0d177f7b6a5a3de";
		shallow = true;
	};

	setupLines = ["*static*"] ++ modules.setupLines ++ [""];

	modules-setup = pkgs.writeText "Setup.local" (builtins.concatStringsSep "\n" setupLines);

	deps = import ./deps args;

	conf-site = pkgs.writeText "conf.site" (builtins.readFile ./conf.site);

	pythonObjs = stdenvNoCC.mkDerivation {
		name = "genvm-cpython-objs";

		outputHash = "sha256-hOy3ReDsp+UdMCckk9A8Tgc/rI0rjpSp5aEoWflEmFs="; # this should not change unless new c native module is added
		outputHashMode = "recursive";

		nativeBuildInputs = with pkgs; [
			gnumake
			perl

			conf-site

			runnersLib.buildPy

			runnersLib.wasi-sdk.package
		];

		srcs = [
			python-source
			deps
		];

		patches = [
			./patch
		];

		postUnpack = ''
			cp ${conf-site} ./conf.site
		'';

		sourceRoot = "cpython-313-src";

		configurePhase = ''
			cat '${modules-setup}' >> Modules/Setup.local

			# otherwise import will fail, it is only part that depends on dynamic linking on import
			perl -i -pe 's/pythonapi = /pythonapi = None #/g' ./Lib/ctypes/__init__.py

			mkdir -p /build/out

			ls ../genvm-cpython-deps

			CONFIG_SITE=../conf.site ${runnersLib.wasi-sdk.env-str} CFLAGS="$CFLAGS -I ../genvm-cpython-deps/include" LDFLAGS="$LDFLAGS  -L../genvm-cpython-deps/lib" ./configure \
				--prefix=/build/out \
				--host=wasm32-wasip1 --build=x86_64-linux-gnu \
				--with-build-python=${runnersLib.buildPy}/bin/python \
				--with-lto \
				--with-ensurepip=no --disable-ipv6 --disable-test-modules \
				--with-tzpath="" --with-doc-strings=false
		'';

		buildPhase = ''
			make -j Programs/python.o libpython3.13.a Modules/_decimal/libmpdec/libmpdec.a Modules/expat/libexpat.a Modules/_hacl/libHacl_Hash_SHA2.a

			make -j inclinstall libinstall

			rm /build/out/lib/python3.13/ctypes/macholib/fetch_macholib
			rm -R /build/out/bin/idle* out/lib/python*/{idlelib,turtledemo} || true
			rm -R /build/out/lib/python*/tkinter || true

			perl -pe 's/\/nix\/store\/[^-]*/\/nix\//g' -i /build/out/lib/python3.13/_sysconfigdata__wasi_wasm32-wasi.py

			(find /build/out -name __pycache__ -print0 | xargs -0 rm -rf) || true
		'';

		installPhase = ''
			mkdir -p "$out/obj"
			mkdir -p "$out/py/"
			mkdir -p "$out/include"

			cp /build/genvm-cpython-deps/lib/*.a "$out/obj/"

			cp -r --dereference /build/genvm-cpython-deps/include/. "$out/include/."

			cp -r /build/out/include/. "$out/include/."
			cp -r /build/out/lib/python3.13/. "$out/py/std"
			cp -r ./obj/. "$out/obj/"

			cp Programs/python.o "$out/obj/"

			find Modules -name '*.a' -print0 | xargs -0 cp -t "$out/obj"

			cp libpython3.13.a "$out/obj/libpython3.13.a"
		'';
	};

	modules = import ./modules (args // { inherit pythonObjs; });

	merged = pkgs.symlinkJoin { name = "genvm-cpython-all-objs"; paths = [pythonObjs] ++ modules.extraObjs; };

	# now link

	runnerJSON = {
		Seq = [
			{ AddEnv = { name = "pwd"; val = "/"; }; }
			{ MapFile = { to = "/py/"; file = "py/"; }; }
			{ AddEnv = { name = "PYTHONHOME"; val = "/py/std"; }; }
			{ AddEnv = { name = "PYTHONPATH"; val = "/py/std:/py/libs"; }; }
			{ When = {
					cond = "det";
					action = {
						Seq = [
							{ Depends = "${runnersLib.hashes.softfloat.uid}"; }
							{ StartWasm = "cpython.det.wasm"; }
						];
					};
				};
			}
			{ When = { cond = "nondet"; action = { StartWasm = "cpython.wasm"; }; }; }
		];
	};

	runnerJSON-file = pkgs.writeText "runner.json" (builtins.toJSON runnerJSON);

	undefinedSymbols = [
		"storage_read"
		"storage_write"
		"get_balance"
		"get_self_balance"
		"gl_call"
	];

	undefinedSymbols-file = pkgs.writeText "undef-symbols.txt" (builtins.concatStringsSep "\n" undefinedSymbols);

	linked = stdenvNoCC.mkDerivation {
		name = "genvm-cpython";
		src = merged;

		phases = ["unpackPhase" "buildPhase" "installPhase"];

		nativeBuildInputs = [
			runnersLib.wasi-sdk.package
			runnersLib.wasmPatchers.floats-2-soft
			runnersLib.wasmPatchers.add-mod-name
		];

		buildPhase = ''
			cp ${undefinedSymbols-file} undef-symbols.txt

			cat <(find obj -name '*.o' | sort) <(find obj -name '*.a' | sort) <(echo -ldl -lwasi-emulated-signal -lwasi-emulated-getpid -lwasi-emulated-process-clocks -lc++ -lc++abi -lc-printscan-long-double) | \
			xargs ${runnersLib.wasi-sdk.env.CC} ${runnersLib.wasi-sdk.env.CFLAGS} -o cpython.raw.wasm \
				-Wl,--stack-first -z stack-size=8388608 \
				-v -Wl,--allow-undefined-file=undef-symbols.txt

			${runnersLib.wasmPatchers.add-mod-name}/bin/genvm-wasm-add-mod-name cpython.raw.wasm cpython.wasm cpython
			${runnersLib.wasmPatchers.floats-2-soft}/bin/genvm-floats-to-soft cpython.wasm cpython.det.wasm
		'';

		installPhase = ''
			mkdir -p "$out"
			cp ./cpython.det.wasm "$out/"
			cp ./cpython.wasm "$out/"
			cp -r ./py "$out"

			cp ${runnerJSON-file} "$out/runner.json"
		'';
	};
in
	[(runnersLib.package {
		inherit (runnersLib.hashes.cpython) id hash;

		baseDerivation = linked;
	})] ++
	modules.runners ++
	[]
