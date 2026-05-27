{ repo ? "https://github.com/genlayerlabs/genvm.git"
, ...
}:
let
	revs = [
		"86b7cce46d9dee4ed1fb76e8107e60617b7622db" # v0.2.0
		"de381dbc862575b2a4f3d43a1b96ec14814af9fd" # v0.2.3
		"fed444c0d9537f41a6ccafeac7c7507a2cd8f69e" # v0.2.4
	];

	mapRev = rev:
		let
			src = builtins.fetchGit {
				url = repo;
				inherit rev;

				shallow = true;
				submodules = true;
			};
		in
			builtins.map (x: x // { inherit rev; }) (import "${src}/runners")
		;
in
	# list[{id, hash, rev, derivation}]
	builtins.concatLists (builtins.map mapRev revs)
