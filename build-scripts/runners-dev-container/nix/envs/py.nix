{ pkgs
, lib
, ...
}:
let
	py = pkgs.python313;
	#.override {
	#	reproducibleBuild = true;
	#	rebuildBytecode = false;
	#	stripTests = true;
	#	stripTkinter = true;
	#	stripIdlelib = true;
	#	static = true;
	#	enableLTO = false;
	#	stdenv = pkgs.stdenv;
	# };
in
{
	inherit py;

	stdenv = pkgs.stdenvNoCC;

	packages = with pkgs; [
		py
		#python313Packages.cython
	];

	env = rec {
	};

	envStr = "";
}
