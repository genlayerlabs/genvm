{ repo ? "https://github.com/genlayerlabs/genvm.git"
, pkgs
, ...
}:
let
	revs = [
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
