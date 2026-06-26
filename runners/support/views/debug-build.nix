{ pkgs
, host-system
, pkgs-overlays ? []
, ...
}:
let
	allRunners = import ../../default.nix { inherit host-system pkgs-overlays; };

	pathOfRunner = runner:
		let
			# the gvm32 hash id (matches `uid` / `hashToIDHash`)
			uidMatch = builtins.match "([^:]+):(.+)" runner.uid;
			hash32 =
				if uidMatch == null
				then throw "invalid runner uid `${runner.uid}`; expected `<prefix>:<hash32>`"
				else builtins.elemAt uidMatch 1;
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
