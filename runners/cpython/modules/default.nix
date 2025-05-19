{ pkgs
, lib
, runnersLib
, ...
}@args:
let
	genvm-ext = import ./_genlayer_wasi args;
	numpy = import ./numpy args;
in {
	extraObjs = genvm-ext.extraObjs ++ numpy.extraObjs;
	runners = genvm-ext.runners ++ numpy.runners;
	setupLines = genvm-ext.setupLines ++ numpy.setupLines;
}
