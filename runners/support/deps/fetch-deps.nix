{ pkgs }:
let
	deps-data = builtins.fromJSON (builtins.readFile ./dependency-urls.json);

	fetch-dep = entry:
		let
			urls = [entry.original_url] ++ entry.alternative_urls;
			hash = entry.hash;
			store-name =
				if entry ? nix_der_name && entry.nix_der_name != null
				then entry.nix_der_name
				else entry.name;
		in
			if entry.fetcher == "fetchurl" then
				pkgs.fetchurl {
					name = store-name;
					inherit urls hash;
				}
			else if entry.fetcher == "fetchzip" then
				pkgs.fetchzip ({
					name = store-name;
					inherit urls hash;
				} // (entry.fetcher_args or {}))
			else
				throw "unknown fetcher: ${entry.fetcher} for ${entry.name}";
	dep-name = entry:
		if entry ? alternative_name && entry.alternative_name != null then entry.alternative_name
		else entry.name;

	included = builtins.filter (entry: dep-name entry != "_") deps-data;

	result = builtins.listToAttrs (builtins.map (entry: {
		name = dep-name entry;
		value = fetch-dep entry;
	}) included);

	must-preload-drvs = builtins.map
		(entry: result.${dep-name entry})
		(builtins.filter (entry: entry.must_preload or false) included);
in
	builtins.foldl' (acc: drv: builtins.seq drv.drvPath acc) result must-preload-drvs
