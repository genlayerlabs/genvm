let
	allRunners = import ./default.nix;
in
	builtins.listToAttrs
		(builtins.map
			(x: let o = builtins.match "([^:]+):(.*)" x.uid; in { name = builtins.head o; value = builtins.head (builtins.tail o); })
			allRunners)
