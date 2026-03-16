# importing this file (no args) results in:
# [{
#   id
#   hash
#   uid
#   derivation # tar file
# }]
let
	pkgs = import
		(builtins.fetchGit {
			url = "https://github.com/NixOS/nixpkgs";
			rev = "2ff43b1d533641116f1740158d121013036a7f74";
			shallow = true;
		})
		{
			system = "x86_64-linux";
		};
	runnersLib = import ./support args;

	args = {
		inherit pkgs runnersLib;
		inherit (pkgs) lib stdenvNoCC;
	};
in
	(import ./py-libs args) ++
	(import ./genlayer-py-std args) ++
	(import ./softfloat args) ++
	(import ./cpython args) ++
	(import ./models args) ++
	[]
