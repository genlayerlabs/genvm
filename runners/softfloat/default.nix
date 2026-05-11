{ pkgs
, stdenvNoCC
, lib
, runnersLib
, pkgs-host
, ...
}@args:
let
	runnerJSON = pkgs-host.writeText "runner.json" (builtins.toJSON { LinkWasm = "softfloat.wasm"; });
in
[
	(runnersLib.package {
		inherit (runnersLib.hashes.softfloat) id hash;

		baseDerivation = stdenvNoCC.mkDerivation {
			name = "softfloat.wasm";

			phases = ["unpackPhase" "buildPhase" "installPhase"];

			src = ./.;

			nativeBuildInputs = [
				runnersLib.wasmPatchers.add-mod-name
				runnersLib.wasi-sdk.package
			];

			buildPhase = ''
				${runnersLib.wasi-sdk.env-str} make -j lib

				${runnersLib.wasmPatchers.add-mod-name}/bin/genvm-wasm-add-mod-name \
					./softfloat-out.wasm \
					./softfloat.wasm \
					softfloat
			'';

			installPhase = ''
				mkdir -p "$out"
				cp ./softfloat.wasm "$out/softfloat.wasm"
				cp "${runnerJSON}" "$out/runner.json"
			'';
		};
	})
]
