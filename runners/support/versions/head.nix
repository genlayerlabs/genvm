{ build-config
, host-system
, deps ? null
, pkgs-overlays ? []
, ...
}:
	builtins.map
		(x: x // { rev = build-config.head-revision; })
		(import ../../default.nix { inherit host-system deps pkgs-overlays; })
