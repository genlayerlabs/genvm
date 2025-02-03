{ pkgs
, lib
, ...
}@args:
pkgs.rustPlatform.buildRustPackage rec {
	pname = "genvm-wasm-add-mod-name";
	version = "0.1.0";
	cargoLock.lockFile = ./Cargo.lock;
	src = pkgs.lib.cleanSource ./.;
}
