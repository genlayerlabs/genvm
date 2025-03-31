{ pkgs
, lib
, ...
}@args:
let
	py = (import ./py args).fullDefault;
	softfloat = import ./softfloat/release.nix args;
in pkgs.stdenvNoCC.mkDerivation {
	name = "genvm-nix-all-runners";

	outputHashMode = "recursive";
	outputHash = "sha256-qqi+HSt/kJFkvGAgc3JT+l3NTNvmPCgSFt4rcJSvmAw="; #lib.fakeHash;

	nativeBuildInputs = [
		py
		softfloat
	];

	phases = [ "installPhase" ];

	installPhase = ''
		mkdir "$out"
		cp --preserve=timestamps --no-preserve=mode,ownership -r "${py.outPath}"/* "$out"
		cp --preserve=timestamps --no-preserve=mode,ownership -r "${softfloat.outPath}"/* "$out"
	'';
}
