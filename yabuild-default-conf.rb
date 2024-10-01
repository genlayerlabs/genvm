{
	profile: "debug",
	wasiSdk: root_src.join('tools', 'downloaded', 'wasi-sdk-24'),
	createTestRunner: true,
	out_dir: root_build.join('out'),
	bin_dir: root_build.join('out', 'bin'),
	runners_dir: root_build.join('out', 'share', 'genvm', 'runners'),
	python: ENV['PYTHON'] || 'python3',
	runners: {
		softfloat: {
			hash: "YF6UZPGJQJBFAJ2GWCINUMKEQIPDZGNGLGPQHEZOEJYAZEYOS6SEYMYQGYDTZ2SR2LXCVW7XM4W5T3DYI3ZRHICY3EFV377OC66JKSI=",
		},
		cpython: {
			hash: "R2N22PSVKTUWQ5APHLEYA2KJKAUMPER4C5WXEP7LWOZFRQZW6RIO3XOOW6EYWGKFFEVL5H5NY4MUC6ET3GMRKS5JICFPNULS5ELMTMQ=",
		},
	}
}.to_ostruct
