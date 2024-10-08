{
	profile: "debug",
	wasiSdk: root_src.join('tools', 'downloaded', 'wasi-sdk-24'),
	createTestRunner: true,
	out_dir: root_build.join('out'),
	bin_dir: root_build.join('out', 'bin'),
	runners_dir: root_build.join('out', 'share', 'genvm', 'runners'),
	runners: {
		softfloat: {
			hash: "YF6UZPGJQJBFAJ2GWCINUMKEQIPDZGNGLGPQHEZOEJYAZEYOS6SEYMYQGYDTZ2SR2LXCVW7XM4W5T3DYI3ZRHICY3EFV377OC66JKSI=",
		},
		cpython: {
			hash: "E45A6QATTSTXX3ECDAXNKNZIMI22KHVHATL5IAPC7GS6WRHRIS7OMXKGIGB7HV72TRGTIDG4WGBHBW2NRLFZKFDZNH7AB4WJLHK2OFA=",
		},
	},

	tools: {
		clang: find_executable("clang") || find_executable("clang-18") || find_executable("clang-17"),
		gcc: find_executable("gcc"),
		mold: find_executable("mold"),
		lld: find_executable("lld"),
		python3: find_executable("python3"),
	},
}.to_ostruct
