# importing this file results in:
# [{
#   id
#   hash
#   uid
#   derivation # tar file
# }]
#
# Callers may pass `pkgs` to evaluate under their own nixpkgs + system;
# if omitted we fall back to a pinned x86_64-linux nixpkgs for legacy
# scripts that used to `import ./runners/default.nix` without args.
{ pkgs ? null
, ...
}:
let
	default-pkgs = import
		(builtins.fetchGit {
			url = "https://github.com/NixOS/nixpkgs";
			rev = "8b27c1239e5c421a2bbc2c65d52e4a6fbf2ff296";
			shallow = true;
		})
		{
			system = "x86_64-linux";
		};

	effective-pkgs = if pkgs == null then default-pkgs else pkgs;

	runnersLib = import ./support args;

	args = {
		pkgs = effective-pkgs;
		inherit runnersLib;
		inherit (effective-pkgs) lib stdenvNoCC;
	};
in
	(import ./py-libs args) ++
	(import ./genlayer-py-std args) ++
	(import ./softfloat args) ++
	(import ./cpython args) ++
	(import ./models args) ++
	[]
