{ conf-target
, name-target
, pkgs
}:
let
	musl-src = builtins.fetchGit {
		url = "https://github.com/kraj/musl.git";
		rev = "0784374d561435f7c787a555aeab8ede699ed298";
		shallow = true;
	};
	aarch64-compiler-rt = builtins.fetchTarball {
		url = "https://storage.googleapis.com/genvm-artifacts/compiler-rt-21.1.5-1-aarch64.pkg.tar.xz";
		sha256 = "08j9n9i7hw6y5bs9mmg0h90g4vr3pbqfijch1gzcwmndm7ndc33k";
	};
in pkgs.stdenvNoCC.mkDerivation {
	name = "libc-${name-target}";

	src = musl-src;

	nativeBuildInputs = [ pkgs.clang pkgs.lld pkgs.coreutils ];

	configurePhase =
	(if name-target == "arm64" then ''
		cp -r ${aarch64-compiler-rt}/. ./compiler-rt
	''else "") +
	''
		CC="clang --target=${conf-target}-linux-musl" \
			CFLAGS="-O2" \
			LDFLAGS="-fuse-ld=lld ${if name-target == "arm64" then "./compiler-rt/usr/lib/clang/21/lib/linux/libclang_rt.builtins-aarch64.a" else ""}" \
			./configure --target=${conf-target}
	'';

	buildPhase = ''
		make -j lib/libc.so
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		cp lib/libc.so "$out/lib/libc.so"
	'';
}
