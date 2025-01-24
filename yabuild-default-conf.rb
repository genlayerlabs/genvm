require 'open3'

extend_config({
	profile: "debug",
	executor_target: nil,
	wasiSdk: root_src.join('tools', 'downloaded', 'wasi-sdk-24'),
	createTestRunner: true,
	out_dir: root_build.join('out'),
	bin_dir: root_build.join('out', 'bin'),
	runners_dir: root_build.join('out', 'share', 'genvm', 'runners'),
	runners: {
		softfloat: {
			hash: "L7KBTDT2B4LHNVA6UAE5BW4BY3EJ5UGQWB34ZQIBWC2TQUCEZLCQ",
		},
		cpython: {
			hash: "6XYGHSRZL2RVN3ZJVHJ7RNY5KIMNRJD7VUQNLIXR2Q2E2CZ4WZQA",
		},
		py_libs: {
			cloudpickle: {
				hash: "JAMDV6TTUV3XLLFHD4XVUNFUIF3GFIEBGALMC3KUT3L6NTLCTDBQ",
			},
			protobuf: {
				hash: "7L6K65E6LQ23GOXLCBEF5DMBG5O4UWEX6FVDQWQ5RKEKBTQOVCCA",
			},
			tiny_onnx_reader: {
				hash: "36TEM4B4HYOZKIWYAEJFKHW7AC24X467MUCJQHNKOB5AMU7VHW6Q",
			},
			word_piece_tokenizer: {
				hash: "Z3CLPEEZIMJM2UJLOK5URKI6LVHNW2YQJPISIQBYPL3HQJRTBNDA",
			},
			genlayermodelwrappers: {
				hash: "test"
			}
		},
		onnx_models: {
			all_MiniLM_L6_v2: {
				hash: "P4HCCVYAVCYECEBHBG5BHTZCIHHONQFG7HQYU4WWCRHNNRXKA2UQ",
			}
		},
	},

	executor: {
		coverage: false,
	},

	tools: {
		clang: find_executable("clang") || find_executable("clang-18") || find_executable("clang-17"),
		gcc: find_executable("gcc"),
		mold: find_executable("mold"),
		lld: find_executable("lld"),
		python3: find_executable("python3"),
	},
})

def run_command_success(*cmd, cwd: nil)
	cmd.map! { |c|
		if c.kind_of?(Pathname)
			c.to_s
		else
			c
		end
	}
	opts = {}
	if not cwd.nil?
		opts[:chdir] = cwd
	end
	std, status = Open3.capture2e(*cmd, **opts)
	raise "command #{cmd} failed with #{std}" if not status.success?
end

root_conf = root_build.join('config')
root_conf.mkpath()

if not config.tools.clang.nil?
	begin
		run_command_success config.tools.clang, '-c', '-o', root_conf.join('a.o'), root_src.join('build-scripts', 'test-tools', 'clang-mold', 'a.c')
		run_command_success config.tools.clang, '-c', '-o', root_conf.join('b.o'), root_src.join('build-scripts', 'test-tools', 'clang-mold', 'b.c')
	rescue => e
		logger.warn("clang doesn't work #{config.tools.clang} #{e}")
		config.tools.clang = nil
	else
		logger.info("clang works")
	end
end
if not config.tools.clang.nil? and not config.tools.mold.nil?
	begin
		run_command_success config.tools.clang, "-fuse-ld=#{config.tools.mold}", '-o', root_conf.join('ab'), root_conf.join('a.o'), root_conf.join('b.o')
		run_command_success root_conf.join('ab')
	rescue => e
		logger.warn("mold doesn't work #{config.tools.mold} #{e}")
		config.tools.mold = nil
	else
		logger.info("mold works")
	end
end
