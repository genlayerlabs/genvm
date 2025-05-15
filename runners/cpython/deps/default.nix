{ pkgs
, ...
}@args:
let
	allDeps =
		builtins.map (x: import x args) [
			./bz2.nix
			./zlib.nix
			./xz.nix
			./ffi
		]
	;
in
	pkgs.symlinkJoin { name = "genvm-cpython-deps"; paths = allDeps; }
