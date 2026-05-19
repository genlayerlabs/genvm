{ build-config
, host-system
, deps ? null
, ...
}:
	builtins.map
		(x: x // { rev = build-config.head-revision; })
		(import ../../default.nix { inherit host-system deps; })
