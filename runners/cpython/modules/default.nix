{ pkgs
, lib
, runnersLib
, ...
}@args:
let
	genvm-ext = import ./_genlayer_wasi args;
	numpy = import ./numpy args;
	pil = import ./pil args;
	all = [genvm-ext numpy pil];
in {
	extraObjs = builtins.concatLists (builtins.map (x: x.extraObjs) all);
	runners = builtins.concatLists (builtins.map (x: x.runners) all);
	setupLines = builtins.concatLists (builtins.map (x: x.setupLines) all);
}
