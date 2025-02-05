{ pkgs
, lib
, pyShell
, ...
}@args:
let
in {
	genvm-wasm-add-mod-name = import ./genvm-wasm-add-mod-name/release.nix args;
	genvm-floats-to-soft = import ./genvm-floats-to-soft/release.nix args;

	genvm-make-runner = pkgs.stdenvNoCC.mkDerivation {
		name = "genvm-make-runner";

		src = ./genvm-make-runner.py;

		phases = ["unpackPhase" "installPhase"];

		buildInputs = [ pyShell.py ];

		unpackPhase = ''
			cp "$src" ./genvm-make-runner.py
		'';

		installPhase = ''
			mkdir -p "$out/bin/"
			echo "#!${pyShell.py}/bin/python3" > "$out/bin/genvm-make-runner.py"
			cat ./genvm-make-runner.py >> "$out/bin/genvm-make-runner.py"
			chmod +x "$out/bin/genvm-make-runner.py"
		'';

		meta.mainProgram = "genvm-make-runner.py";
	};

	genvm-py-precompile = pkgs.stdenvNoCC.mkDerivation {
		name = "genvm-py-precompile";

		src = ./genvm-py-precompile.py;

		phases = ["unpackPhase" "installPhase"];

		buildInputs = [ pyShell.py ];

		unpackPhase = ''
			cp "$src" ./genvm-py-precompile.py
		'';

		installPhase = ''
			mkdir -p "$out/bin/"
			echo "#!${pyShell.py}/bin/python3" > "$out/bin/genvm-py-precompile.py"
			cat ./genvm-py-precompile.py >> "$out/bin/genvm-py-precompile.py"
			chmod +x "$out/bin/genvm-py-precompile.py"
		'';

		meta.mainProgram = "genvm-py-precompile.py";
	};
}
