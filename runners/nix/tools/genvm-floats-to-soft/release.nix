{ pkgs
, lib
, ...
}@args:
pkgs.rustPlatform.buildRustPackage rec {
	pname = "genvm-floats-to-soft";
	version = "0.1.0";
	cargoLock.lockFile = ./Cargo.lock;
	src = pkgs.lib.cleanSource ./.;
}
