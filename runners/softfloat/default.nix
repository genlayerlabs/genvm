{ pkgs
, stdenvNoCC
, lib
, runnersLib
, ...
}@args:
let
	runnerJSON = pkgs.writeText "runner.json" (builtins.toJSON { LinkWasm = "softfloat.wasm"; });
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
				pkgs.wabt
			];

			buildPhase = ''
				${runnersLib.wasi-sdk.env-str} make -j lib

				echo "running wasm-opt"
				wasm-opt -g -O3 ${runnersLib.wasi-sdk.env.WASMOPTFLAGS} -o ./softfloat-opt.wasm ./softfloat-out.wasm

				cp ./softfloat-out.wasm ./softfloat-opt.wasm

				${runnersLib.wasmPatchers.add-mod-name}/bin/genvm-wasm-add-mod-name \
					./softfloat-opt.wasm \
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
