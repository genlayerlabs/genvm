# importing this file ({} no args) results in:
# [{
#   id
#   hash
#   uid
#   derivation # tar file
# }]
# all args are optional
{ host-system ? "x86_64-linux"
	, deps ? null
	, ...
}:
let
	pkgs-pure =
		(builtins.fetchGit {
			url = "https://github.com/NixOS/nixpkgs";
			rev = "2ff43b1d533641116f1740158d121013036a7f74";
			shallow = true;
		});
	pkgs = import pkgs-pure {
		system = "x86_64-linux";
	};
	pkgs-host = import pkgs-pure {
		system = host-system;
	};
	runnersLib = import ./support args;

	my-deps = import ./support/deps/fetch-deps.nix { pkgs = pkgs-host; };

	real-deps =
		if deps == null
		then my-deps
		else deps;

	args = {
		inherit pkgs pkgs-host runnersLib;
		inherit (pkgs) lib stdenvNoCC;

		deps = real-deps;
	};
in
	(import ./py-libs args) ++
	(import ./genlayer-py-std args) ++
	(import ./softfloat args) ++
	(import ./cpython args) ++
	(import ./models args) ++
	[]
