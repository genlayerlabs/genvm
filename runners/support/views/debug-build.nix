{ pkgs
, host-system
, pkgs-overlays ? []
, ...
}:
let
	allRunners = import ../../default.nix { inherit host-system pkgs-overlays; };

	pathOfRunner = runner:
		let
			hash32 =
				if runner.hash == "test"
				then "test"
				else builtins.convertHash { hash = runner.hash; toHashFormat = "nix32"; };
		in "${runner.id}/${builtins.substring 0 2 hash32}/${builtins.substring 2 50 hash32}.tar";

	installLines =
		builtins.concatLists
			(builtins.map
				(x: ["mkdir -p $out/$(dirname -- ${pathOfRunner x})" "cp ${x.derivation} $out/${pathOfRunner x}"])
				allRunners);
in pkgs.stdenvNoCC.mkDerivation {
	name = "genvm-debug-runners";
	phases = ["installPhase"];

	installPhase = builtins.concatStringsSep "\n" (installLines ++ [""]);
}
