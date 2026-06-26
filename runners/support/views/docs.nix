{ commitToTagStr, build-config-str }:

let
	build-config = builtins.fromJSON build-config-str;
	commitToTag = builtins.fromJSON commitToTagStr;
	# list[{id, hash, derivation}]
	allRunnersList = import ../versions/all.nix { inherit build-config; };
	res = builtins.foldl' (l: r:
		let
			rev_id = if builtins.hasAttr r.rev commitToTag then commitToTag.${r.rev} else r.rev;
			old_l_elem = if builtins.hasAttr rev_id l then l.${rev_id} else {};
			old_l_id = if builtins.hasAttr r.id old_l_elem then old_l_elem.${r.id} else {};

			# gvm32 (Crockford Base32) — the encoding the executor uses for runner
			# paths. Extract it from `uid` (`id:gvm32hash`); NOT Nix base32.
			r_hash = if r.hash == "test" then "vTEST" else builtins.head (builtins.match "[^:]+:(.*)" r.uid);

			new_l_id = old_l_id // { ${r_hash} = true; };
			new_l_elem = old_l_elem // { ${r.id} = new_l_id; };
		in
			l // { ${rev_id} = new_l_elem; }
	) {} allRunnersList;
in
	builtins.mapAttrs (name: builtins.mapAttrs (name: builtins.attrNames)) res
