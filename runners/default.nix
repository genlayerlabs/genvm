# importing this file (no args) results in:
# [{
#   id
#   hash
#   uid
#   derivation # tar file
# }]
let
	nixpkgs = import (fetchTarball { url = "https://github.com/NixOS/nixpkgs/archive/nixos-24.11.tar.gz"; sha256 = "1cvxfj03xhakyrrz8bh4499vz5d35ay92575irrbmydcxixsrf3w"; });
	pkgs = nixpkgs {
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
