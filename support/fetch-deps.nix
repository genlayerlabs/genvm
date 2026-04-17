{ pkgs }:
let
	deps-data = builtins.fromJSON (builtins.readFile ./dependencies/dependency-urls.json);

	fetch-dep = entry:
		let
			urls = [entry.original_url] ++ entry.alternative_urls;
			hash = entry.hash;
		in
			if entry.fetcher == "fetchurl" then
				pkgs.fetchurl {
					name = entry.name;
					inherit urls hash;
				}
			else if entry.fetcher == "fetchzip" then
				pkgs.fetchzip ({
					name = entry.name;
					inherit urls hash;
				} // (if entry ? extension then { extension = entry.extension; } else {}))
			else
				throw "unknown fetcher: ${entry.fetcher} for ${entry.name}";
	dep-name = entry:
		if entry ? alternative_name && entry.alternative_name != null then entry.alternative_name
		else entry.name;

	included = builtins.filter (entry: dep-name entry != "_") deps-data;
in
	builtins.listToAttrs (builtins.map (entry: {
		name = dep-name entry;
		value = fetch-dep entry;
	}) included)
