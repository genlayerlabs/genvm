{ pkgs
, deps
, zig
, ...
}:
let
	iconv-src = deps."libiconv-1.18";
in pkgs.stdenvNoCC.mkDerivation {
	name = "libiconv";

	src = iconv-src;

	nativeBuildInputs = [ zig pkgs.coreutils ];

	configurePhase = ''
		CC=zig-cc-arm64-macos \
			LD=zig-cc-arm64-macos \
			AR="${zig}/zig ar" \
			CFLAGS="-O2" \
			LDFLAGS="-Wl,-install_name,@rpath/libiconv.dylib" \
			./configure --host=aarch64-apple-darwin --enable-shared=yes
	'';

	buildPhase = ''
		make -j
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		cp lib/.libs/libiconv.dylib "$out/lib/libiconv.dylib"
	'';
}
