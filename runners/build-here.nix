let
	nixpkgs = import (fetchTarball { url = "https://github.com/NixOS/nixpkgs/archive/nixos-24.11.tar.gz"; sha256 = "1cvxfj03xhakyrrz8bh4499vz5d35ay92575irrbmydcxixsrf3w"; });
	pkgs = nixpkgs {
		system = "x86_64-linux";
	};

	allRunners = import ./default.nix;

	pathOfRunner = runner:
		let hash32 = builtins.convertHash { hash = runner.hash; toHashFormat = "nix32"; }; in
		"${runner.id}/${hash32}.tar";

	installLines =
		builtins.concatLists
			(builtins.map
				(x: ["mkdir -p \"$out/${x.id}\"" "cp ${x.derivation} $out/${pathOfRunner x}"])
				allRunners);
in pkgs.stdenvNoCC.mkDerivation {
	name = "genvm-test-runners";

	#nativeBuildInputs = builtins.map (x: x.derivation) allRunners;

	phases = ["installPhase"];

	installPhase = builtins.concatStringsSep "\n" (installLines ++ [""]);
}
