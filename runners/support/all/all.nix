{ build-config
, ...
}@args:
let
	gen-00 = import ./generation-00.nix { repo = build-config.repo-url; };
	head = import ./head.nix args;
in gen-00 ++ head
