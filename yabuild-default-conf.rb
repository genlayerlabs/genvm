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
		py_libs: {
			cloudpickle: {
				hash: "B6PA2XEWJERJLTPOG525576Y3MBMUTJDK5AVS542SMCJO5MR3GUQ",
			},
			protobuf: {
				hash: "N2WIUENBHXMWXCL53X6ESRT7O3MTVHZMRRLMAQZY72T44IPTFNXQ",
			},
			tiny_onnx_reader: {
				hash: "DVGEU3ON7UGZSGC4CUIBGR4ZY2JMAGHKFY3UR6FO6DRZ52QLLVPQ",
			},
			word_piece_tokenizer: {
				hash: "RYS4SQP7KEP7ZMQEDXSZIBOPERNPH2B3WVQ3ISLMV4EFMU34LU3A",
			},
			genlayermodelwrappers: {
				hash: "test"
			}
		},
		onnx_models: {
			all_MiniLM_L6_v2: {
				hash: "TC2HXK7ZP6Z6YHNTZZ65MHJRHRAUFKOUFWDC4NFAODV2JC72AGDA",
			}
		},
		**JSON.load_file(root_src.join('runners/nix/hashes.json')),
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
