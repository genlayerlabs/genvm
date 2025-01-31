{ pkgs
, lib
, wasmShell
, pyShell
, ...
}@args:
let
	pyDeps = import ./deps args;
in
wasmShell.stdenv.mkDerivation {
	pname = "genvm-cpython";
	version = "3.13";

	outputHash = lib.fakeHash;
	outputHashMode = "recursive";

	nativeBuildInputs = [ pyDeps pkgs.perl ] ++ wasmShell.packages;

	src = pyShell.py.src;

	configurePhase = ''
		export ${wasmShell.envStr}
		export CFLAGS="$CFLAGS -I${pyDeps.outPath}/include"
		export CONFIG_SITE="$(readlink -f Tools/wasm/config.site-wasm32-wasi)"
		export LDFLAGS="-L${pyDeps.outPath}/lib"

		mkdir -p /build/out

		./configure \
			--prefix "/build/out" \
			--host=wasm32-wasip1 --build=x86_64-linux-gnu \
			--with-build-python=${pyShell.py.outPath}/bin/python \
			--with-ensurepip=no --disable-ipv6 --disable-test-modules
	'';

	buildPhase = ''
		make -j

		make install

		rm -R /build/out/bin/idle* /build/out/lib/python*/{idlelib,turtledemo} || true
		rm -R /build/out/lib/python*/tkinter || true
		find /build/out -type f -name '*.pyc' -delete

		find /build/out -type f -exec grep -Iq . {} \; -print0 | xargs -0 perl -pe 's/\/nix\/store[^\-]*-/\//g' -i

		${pyShell.py.outPath}/bin/python -m compileall --invalidation-mode unchecked-hash /build/out

	'';

	installPhase = ''
		set -ex
		mkdir -p "$out"
		cp -r /build/out/lib/python* "$out/std"
		cp /build/out/bin/python3.wasm "$out/cpython.wasm"

		ls /bin || true
		grep -rnP '/(nix|tmp|build)/' "$out" || true
	'';

	dontPatchELF = true;
	dontFixup = true;
}
