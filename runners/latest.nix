let
	allRunners = import ./default.nix;
	interesting = [
		"py-genlayer"
		"py-genlayer-multi"
		"softfloat"
		"cpython"
	];
in
	builtins.listToAttrs
		(builtins.map
			(x: let o = builtins.match "([^:]+):(.*)" x.uid; in { name = builtins.head o; value = builtins.head (builtins.tail o); })
			(builtins.filter
				(x: builtins.any (v: v == x.id) interesting)
				allRunners))
