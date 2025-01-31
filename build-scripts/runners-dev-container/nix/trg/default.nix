{ pkgs
, lib
, ...
}@args:
let
	py = import ./py args;
in
	py
