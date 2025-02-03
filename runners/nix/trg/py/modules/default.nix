{ pkgs
, lib
, wasmShell
, pyShell
, rustShell
, ...
}@args:{
	genvm-ext = import ./genvm-cpython-ext/release.nix args;
	numpy = import ./numpy args;
}
