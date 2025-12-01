{ pkgs
, build-config
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

	latest = builtins.toFile "latest.json" (builtins.toJSON (converter-single (import ./default.nix)));
	all = builtins.toFile "all.json" (builtins.toJSON (converter-multi (import ./support/all/all.nix args)));

	subpath = "executor/${build-config.executor-version}/data/";
in {
	universal-manifest = {
		runners-latest = pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-runners-latest";

			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;
			dontFixup = true;

			installPhase = ''
				mkdir -p $out/${subpath}
				cp ${latest} $out/${subpath}/latest.json
			'';
		};

		runners-all = pkgs.stdenvNoCC.mkDerivation rec {
			name = "genvm-runners-all";

			dontUnpack = true;
			dontConfigure = true;
			dontBuild = true;
			dontFixup = true;

			installPhase = ''
				mkdir -p $out/${subpath}
				cp ${all} $out/${subpath}/all.json
			'';
		};
	};
}
