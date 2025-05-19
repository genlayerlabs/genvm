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
			rev = "8b27c1239e5c421a2bbc2c65d52e4a6fbf2ff296";
			shallow = true;
		})
		{
			system = "x86_64-linux";
		};
	runnersLib = import ./support args;

	args = {
		inherit pkgs runnersLib;
		inherit (pkgs) lib stdenvNoCC;

		genVMAllowTest = import ./dbg.nix;
	};
in
	(import ./py-libs args) ++
	(import ./genlayer-py-std args) ++
	(import ./softfloat args) ++
	(import ./cpython args) ++
	(import ./models args) ++
	[]
