{ pkgs
, lib
, ...
}:
let
	py = pkgs.python313;
	cython = pkgs.python313Packages.cython;
in {
	inherit py cython;
	all = [ py cython ];
}
