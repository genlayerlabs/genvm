{ build-config
, ...
}:
	builtins.map
		(x: x // { rev = build-config.head-revision; })
		(import ../default.nix)
