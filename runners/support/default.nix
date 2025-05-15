{ pkgs
, stdenvNoCC
, ...
}@args:
rec {
	hashes = import ../hashes.nix;

	wasi-sdk = import ./wasi-sdk.nix args;

	wasmPatchers = {
		floats-2-soft = import ./genvm-floats-to-soft args;
		add-mod-name = import ./genvm-wasm-add-mod-name args;
	};

	hashToIDHash = hash: builtins.convertHash { inherit hash; toHashFormat = "nix32"; };
	package = { id, hash, baseDerivation }: {
		inherit id hash;

		uid = "${id}:${hashToIDHash hash}";

		derivation = stdenvNoCC.mkDerivation {
			name = "genvm_runner_${id}_${hashToIDHash hash}";

			srcs = [ baseDerivation ./build-scripts ];
			sourceRoot = ".";

			phases = ["unpackPhase" "installPhase"];

			nativeBuildInputs = with pkgs; [ python313 ];

			installPhase = ''
				${pkgs.python313}/bin/python3 ./build-scripts/make-tar.py
			'';

			outputHashMode = "flat";
			outputHash = hash;
		};
	};

	packageWithRunnerJSON = { id, hash, baseDerivation, expr }: package {
		inherit id hash;

		baseDerivation = pkgs.symlinkJoin {
			name = "genvm_runner_${id}_${hashToIDHash hash}-merged";
			paths = [
				baseDerivation
				(let file = pkgs.writeText "runner.json" (builtins.toJSON expr); in stdenvNoCC.mkDerivation {
					name = "genvm_runner_${id}_${hashToIDHash hash}-runner";

					phases = ["installPhase"];
					installPhase = ''
						mkdir -p "$out"
						cp "${file}" "$out/runner.json"
					'';
				})
			];
		};
	};

	packageGlue = { id, hash, expr }: package {
		inherit id hash;

		baseDerivation = let file = pkgs.writeText "runner.json" (builtins.toJSON expr); in stdenvNoCC.mkDerivation {
			name = "genvm_runner_${id}_${hashToIDHash hash}-runner";

			phases = ["installPhase"];
			installPhase = ''
				mkdir -p "$out"
				cp "${file}" "$out/runner.json"
			'';
		};
	};

	buildPy = pkgs.python313;

	toListExcluded = info: item: if info.excludeFromBuild then [] else [item];
}
