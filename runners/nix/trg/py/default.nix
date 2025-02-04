{ pkgs
, lib
, wasmShell
, pyShell
, rustShell
, tools
, runnerHashes
, ...
}@args:
let
	pyDeps = import ./deps args;
	pyModules = import ./modules args;
in rec {
	configurator = {
		nativeBuildInputs = [ pyDeps pkgs.perl ];
		version = "3.13";

		src = pyShell.py.src;

		configurePhase = ''
			perl -i -pe 's/pythonapi = /pythonapi = None #/g' ./Lib/ctypes/__init__.py

			export ${wasmShell.envStr}
			export CFLAGS="$CFLAGS -I${pyDeps.outPath}/include"
			export CONFIG_SITE="$(readlink -f Tools/wasm/config.site-wasm32-wasi)"
			export LDFLAGS="-L${pyDeps.outPath}/lib -Lgenvm-extra"

			readlink -f .
			mkdir -p /build/out

			./configure \
				--prefix "/build/out" \
				--host=wasm32-wasip1 --build=x86_64-linux-gnu \
				--with-build-python=${pyShell.py.outPath}/bin/python \
				--with-ensurepip=no --disable-ipv6 --disable-test-modules
		'';
	};

	runnerJson = {
		Seq = [
			{ AddEnv = { name = "pwd"; val = "/"; }; }
			{ MapFile = { to = "/py/std"; file = "std/"; }; }
			{ AddEnv = { name = "PYTHONHOME"; val = "/py/std"; }; }
			{ AddEnv = { name = "PYTHONPATH"; val = "/py/std:/py/libs"; }; }
			{ When = {
					cond = "det";
					action = {
						Seq = [
							{ Depends = "softfloat:${runnerHashes.softfloat}"; }
							{ StartWasm = "cpython.wasm"; }
						];
					};
				};
			}
			{ When = { cond = "nondet"; action = { StartWasm = "cpython.nondet.wasm"; }; }; }
		];
	};

	fullDefault = wasmShell.stdenv.mkDerivation (configurator // {
		pname = "genvm-cpython";

		outputHash = "sha256-czL3TCzXLVN7PCXppGWfujOm6gYDVBLsSM0aOHjbauw="; #lib.fakeHash;
		outputHashMode = "recursive";

		nativeBuildInputs = configurator.nativeBuildInputs ++
			[ pkgs.perl
				tools.genvm-floats-to-soft tools.genvm-wasm-add-mod-name
				tools.genvm-make-runner tools.genvm-py-precompile
				pyModules.genvm-ext pyModules.numpy
				wasmShell.sdk
			];

		buildPhase = ''
			mkdir -p genvm-extra/
			cp "${pyModules.genvm-ext}/lib"/* genvm-extra/
			cp "${pyModules.numpy}/lib"/* genvm-extra/

			echo '*static*' >> Modules/Setup.local
			cat "${pyModules.genvm-ext}/Setup.local" >> Modules/Setup.local
			cat "${pyModules.numpy}/Setup.local" >> Modules/Setup.local

			make -j

			make install

			rm -R /build/out/bin/idle* /build/out/lib/python*/{idlelib,turtledemo} || true
			rm -R /build/out/lib/python*/tkinter || true
			find /build/out -type f -name '*.pyc' -delete

			find /build/out -type f -exec grep -Iq . {} \; -print0 | xargs -0 perl -pe 's/\/nix\/store[^\-]*-/\//g' -i

			mkdir genvm-export
			genvm-wasm-add-mod-name /build/out/bin/python3.wasm  genvm-export/cpython.nondet.wasm cpython
			genvm-floats-to-soft genvm-export/cpython.nondet.wasm genvm-export/cpython.wasm

			mkdir -p genvm-export/std
			cp --preserve=timestamps --no-preserve=mode,ownership -r /build/out/lib/python*/* genvm-export/std
			cp --preserve=timestamps --no-preserve=mode,ownership  -r "${pyModules.numpy}/py"/* genvm-export/std

			chmod -R +w genvm-export/std

			genvm-py-precompile.py genvm-export/std

			mkdir -p genvm-export-runner
			echo '${builtins.toJSON runnerJson}' > genvm-export/runner.json
			genvm-make-runner.py \
				--expected-hash ${runnerHashes.cpython} \
				--out-dir genvm-export-runner/cpython \
				--src-dir genvm-export
		'';

		installPhase = ''
			mkdir -p "$out/share/genvm/runners"
			cp --preserve=timestamps --no-preserve=mode,ownership -r genvm-export-runner/* "$out/share/genvm/runners"
		'';
	});
}
