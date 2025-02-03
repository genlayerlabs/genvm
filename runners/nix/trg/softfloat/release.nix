{ pkgs
, lib
, wasmShell
, tools
, runnerHashes
, ...
}@args:
let
	runner_config = ''
		{"out_dir": "export/softfloat", "files":[{"path":"softfloat.wasm","read_from":"softfloat-patched.wasm"},{"path":"runner.json","read_from":"runner.json"}],"expected_hash":"${runnerHashes.softfloat}"}
	'';
in
wasmShell.stdenv.mkDerivation {
	pname = "genvm-softfloat";
	version = "0.0.1";

	outputHash = "sha256-cwNr96KJlfjICq0b/s52qI2tZ9IA2VQQjEs7vOo6GNQ=";
	outputHashMode = "recursive";

	src = ./.;

	nativeBuildInputs = [ tools.genvm-wasm-add-mod-name tools.genvm-make-runner wasmShell.sdk ];

	phases = ["unpackPhase" "buildPhase" "installPhase"];

	buildPhase = ''
		export PATH="${wasmShell.path}:$PATH"
		export CFLAGS="${wasmShell.env.CFLAGS}"
		make -j lib
		genvm-wasm-add-mod-name ./softfloat.wasm ./softfloat-patched.wasm softfloat

		genvm-make-runner.py \
				--expected-hash ${runnerHashes.softfloat} \
				--out-dir export/softfloat \
				--src-dir . \
				--config runner-config.json
	'';

	installPhase = ''
		mkdir -p "$out/share/genvm/runners"
		cp --preserve=timestamps --no-preserve=mode,ownership -r export/* "$out/share/genvm/runners"
	'';
}
