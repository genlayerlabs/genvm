{ pkgs
, root-src
}:
let
	mergeDoubleDepthAttrs = l: r:
		let
			allKeysWithDups = builtins.attrNames l ++ builtins.attrNames r;
			allKeys = builtins.attrNames (builtins.listToAttrs (builtins.map (x: { name = x; value = true; }) allKeysWithDups));
		in
			builtins.listToAttrs (builtins.map (k: {
				name = k;
				value = (if builtins.hasAttr k l then l.${k} else {}) //
					(if builtins.hasAttr k r then r.${k} else {});
			}) allKeys);

	zip-lists = list1: list2:
		let
			len1 = builtins.length list1;
			len2 = builtins.length list2;
			minLen = if len1 < len2 then len1 else len2;
		in
			builtins.genList (i: [
				(builtins.elemAt list1 i)
				(builtins.elemAt list2 i)
			]) minLen;

	trace-echo = x: builtins.trace x x;

	git-third-party-data = (builtins.fromJSON (builtins.readFile ../.git-third-party/config.json)).repos;
	extra-repos = builtins.map (x: {
		name = x;
		drv =
			let
				unpatched = builtins.fetchGit {
					url = git-third-party-data.${x}.url;
					rev = git-third-party-data.${x}.commit;
					shallow = true;

					name = "gtt-" + builtins.hashString "sha256" x + "-unpatched";
				};
			in
				pkgs.applyPatches {
					name = "gtt-" + builtins.hashString "sha256" x + "-patched";
					src = unpatched;
					patches = builtins.genList (patch-no: ../.git-third-party/patches/${x}/${toString (patch-no + 1)}) git-third-party-data.${x}.patches;
				};
	}) (builtins.attrNames git-third-party-data);
	full-src = pkgs.stdenvNoCC.mkDerivation {
		name = "genvm-full-src";
		srcs = [
			root-src
		] ++ builtins.map (x: x.drv) extra-repos;
		sourceRoot = ".";

		dontUnpack = true;
		dontConfigure = true;
		dontBuild = true;

		installPhase = ''
			echo "Starting to install"
			mkdir -p "$out"
			cp --no-preserve=ownership -r ${root-src}/. "$out/."
			chmod -R u+w "$out"
		'' + builtins.concatStringsSep "\n" (builtins.map (x: ''
			mkdir -p "$out/${x.name}"
			cp --no-preserve=ownership -r ${x.drv}/. "$out/${x.name}/."
		'') extra-repos);

		dontFixup = true;
	};
in rec {
	inherit pkgs;

	root-src = full-src;

	zig = import ./zig.nix { inherit pkgs; };

	get-root-subtree = paths:
		let
			paths-split = builtins.map (p: pkgs.lib.splitString "/" p) paths;
		in pkgs.lib.cleanSourceWith {
			src = full-src;
			filter = path: type:
				let
					relPath = pkgs.lib.removePrefix (toString full-src + "/") (toString path);
					relPathSplit = pkgs.lib.splitString "/" relPath;
					allow-dflt = pkgs.lib.any
						(prefix: builtins.all
							(l_r: (builtins.head l_r) == (builtins.elemAt l_r 1))
							(zip-lists relPathSplit prefix))
						paths-split;
				in
					if pkgs.lib.hasSuffix ".nix" relPath
					then false
					else
						allow-dflt;
						# builtins.trace "${relPath} -> ${if allow-dflt then "true" else "false"}" allow-dflt;
		};

	compile-rust = import ./compile-rust.nix { inherit pkgs zig; };
	merge-components = builtins.foldl' mergeDoubleDepthAttrs {};
}
