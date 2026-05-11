{ pkgs
, stdenvNoCC
, pkgs-host
, ...
}@args:
let
	dev-mode = import ./versions/dev-mode.nix;
in rec {
	hashes = import ./versions/current.nix;

	wasi-sdk = import ./tools/wasi-sdk.nix args;

	wasmPatchers = {
		floats-2-soft = import ./tools/genvm-floats-to-soft args;
		add-mod-name = import ./tools/genvm-wasm-add-mod-name args;
	};

	hashToIDHash = hash: if hash == "test" then "test" else builtins.convertHash { inherit hash; toHashFormat = "nix32"; };
	package = { id, hash, baseDerivation }: {
		inherit id hash;

		uid = "${id}:${hashToIDHash hash}";

		derivation = pkgs-host.stdenvNoCC.mkDerivation ({
			name = "genvm_runner_${id}_${hashToIDHash hash}";

			srcs = [ baseDerivation ./scripts ];
			sourceRoot = ".";

			phases = ["unpackPhase" "buildPhase" "installPhase"];

			nativeBuildInputs = with pkgs-host; [ python313 ];

			buildPhase = ''
				out="$(readlink -f ./scripts)/debug-out.tar" ${pkgs-host.python313}/bin/python3 ./scripts/make-tar.py
			'';

			installPhase = ''
				${pkgs-host.python313}/bin/python3 ./scripts/make-tar.py
			'';

			outputHashMode = "flat";
		} // (if hash == "test" then assert dev-mode; {} else { outputHash = hash; }));
	};

	packageWithRunnerJSON = { id, hash, baseDerivation, expr }: package {
		inherit id hash;

		baseDerivation = pkgs-host.symlinkJoin {
			name = "genvm_runner_${id}_${hashToIDHash hash}-merged";
			paths = [
				baseDerivation
				(let file = pkgs-host.writeText "runner.json" (builtins.toJSON expr); in pkgs-host.stdenvNoCC.mkDerivation {
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

		baseDerivation = let file = pkgs-host.writeText "runner.json" (builtins.toJSON expr); in pkgs-host.stdenvNoCC.mkDerivation {
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
