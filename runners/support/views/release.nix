{ pkgs
, host-system
, ...
}@args:
let
	converter-single = list-of-runners: builtins.listToAttrs
		(builtins.map
			(x: let o = builtins.match "([^:]+):(.*)" x.uid; in { name = builtins.head o; value = builtins.head (builtins.tail o); })
			list-of-runners);

	converter-multi = list-of-runners:
		let
			# Extract key-value pairs from the list
			pairs = builtins.map
				(x: let o = builtins.match "([^:]+):(.*)" x.uid; in {
					name = builtins.head o;
					value = builtins.head (builtins.tail o);
				})
				list-of-runners;

			# Group values by key
			groupByKey = pairs:
				builtins.foldl'
					(acc: pair:
						let
							existing = acc.${pair.name} or [];
							newValue = if builtins.elem pair.value existing then existing else existing ++ [pair.value];
						in acc // { ${pair.name} = newValue; }
					)
					{}
					pairs;
		in groupByKey pairs;

	dflt-base = import ../../default.nix;
	dflt =
		if builtins.isFunction dflt-base
		then dflt-base args
		else dflt-base;
in {
	latest = builtins.toFile "latest.json" (builtins.toJSON (converter-single dflt));
	all = builtins.toFile "all.json" (builtins.toJSON (converter-multi (import ../versions/all.nix args)));
}
