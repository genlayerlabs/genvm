{ repo ? "https://github.com/genlayerlabs/genvm.git"
, ...
}:
let
	revs = [
		"b096281821f025289fd99f29a8f897a1e553187d"
		"8a404a0b5a6ddc67da763da23222f5626bd0f937"
		"018fdd7ab3f52e556bc86fdb6fe3cd3f910d90f0"
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
