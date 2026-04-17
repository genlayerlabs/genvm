{ pkgs }:
let
	deps-data = builtins.fromJSON (builtins.readFile ./dependencies/dependency-urls.json);

	fetch-dep = entry:
		let
			has-alt = entry.alternative_urls != [];
			urls = if has-alt then [entry.original_url] ++ entry.alternative_urls else null;
			hash-is-sri = builtins.substring 0 7 entry.hash == "sha256-";
			hash-attr = if hash-is-sri then { hash = entry.hash; } else { sha256 = entry.hash; };
		in
			if entry.fetcher == "fetchurl" then
				builtins.fetchurl {
					url = entry.original_url;
					sha256 = entry.hash;
				}
			else if entry.fetcher == "fetchTarball" then
				builtins.fetchTarball {
					url = entry.original_url;
					sha256 = entry.hash;
				}
			else
				pkgs.fetchzip (hash-attr // {
					name = entry.name;
				} // (if has-alt then { inherit urls; } else { url = entry.original_url; })
				// (if entry ? extension then { extension = entry.extension; } else {}));
	dep-name = entry:
		if entry ? alternative_name && entry.alternative_name != null then entry.alternative_name
		else entry.name;

	included = builtins.filter (entry: dep-name entry != "_") deps-data;
in
	builtins.listToAttrs (builtins.map (entry: {
		name = dep-name entry;
		value = fetch-dep entry;
	}) included)
