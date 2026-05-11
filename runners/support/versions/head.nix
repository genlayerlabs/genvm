{ build-config
, host-system
, ...
}:
	builtins.map
		(x: x // { rev = build-config.head-revision; })
		(import ../../default.nix { inherit host-system; })
