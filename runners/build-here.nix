let
	pkgs = import
		(builtins.fetchGit {
			url = "https://github.com/NixOS/nixpkgs";
			rev = "8b27c1239e5c421a2bbc2c65d52e4a6fbf2ff296";
			shallow = true;
		})
		{
			system = "x86_64-linux";
		};

	allRunners = import ./default.nix;

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
	name = "genvm-test-runners";
	phases = ["installPhase"];

	installPhase = builtins.concatStringsSep "\n" (installLines ++ [""]);
}
