{ name-target
, pkgs
, zig
}:
let
	lua-src = builtins.fetchGit {
		url = "https://github.com/lua/lua.git";
		rev = "75ea9ccbea7c4886f30da147fb67b693b2624c26";
		shallow = true;
	};

	version = "5.3";

	isMacos = name-target == "macos-arm64";

	outSuffix = if isMacos then "dylib" else "so";
in pkgs.stdenvNoCC.mkDerivation {
	name = "liblua-${name-target}";

	inherit version;

	src = lua-src;

	nativeBuildInputs = [ zig ] ++ (if isMacos then [ pkgs.pkgsCross.aarch64-darwin.buildPackages.stdenv.cc ] else []);

	doNotConfigure = true;

	buildPhase = ''
		set -e

		export SOURCE_DATE_EPOCH=1609459200

		for i in ./*.c ; do
			zig-cc-${name-target} ${if isMacos then "-g0" else ""} -O2 -fPIC -I. -fdebug-prefix-map=${toString zig}=/zig -no-canonical-prefixes -c "$i" -o "$i.o"
		done

		ls *.o | sort | xargs zig-cc-${name-target} --verbose -O2 -fPIC -shared -o liblua.${outSuffix}
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		echo "${toString zig}"
		cp liblua.${outSuffix} "$out/lib/liblua-${version}.${outSuffix}"
	'';
}
