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
			hash: "5VML6RYPX3UU3GOE4IESJLJLWHUOTSJK3M6XUEXEL3HDA65DZWCY2YMZ4MYIRGTQKZEDZXAA2X57RA4AMCGV4IK4EF5CKITMTTXWXEQ=",
		},
		cpython: {
			hash: "FFAO5GX6KLNP3JQPCU2LUMSJX3LX7A6AMNHMZH5N25DJM35H2REAXUWZWSHAJGI3WLESKEUDXVDAADMK447FABTHVY552G63T2JJBFQ=",
		},
		py_libs: {
			cloudpickle: {
				hash: "SZ2YUZ4IBHUOFTOHM3DOANHHQEGQKYCHJI3IRLCAVYBQR632BAO2BIFWQR2433L3E4LLWNLV2TP6AT7X2ZAXJSXWEYRJ2U24XMXASOQ=",
			},
			protobuf: {
				hash: "PKNSNRCGBCY5LENTRXL36RPEX2U3DKW72ZWU6GH6NEDVRLX4RP5TH4ZMEDPUN4XQEFTALAXIU4YB7PMWBHPZ47OS43LRM33AHAC3X5Y=",
			},
			tiny_onnx_reader: {
				hash: "I7PH6GIHME75FGWZXGUUWYQXOZ2LEYIOUUE2GPKGMF6G7V3AXZV5D2EAYRQEI2ZHGNGZ3AGW5CJS324TOWH3EMEITN25NQ7PEBWN54Y=",
			},
			word_piece_tokenizer: {
				hash: "MFEYZCJF54QKSMWNJAASCF4G3SFW22JRZPZ4U67YFVKV33QTWUGA5YKZ65RZS3JUC4JZLVSTYW2CKBLA5T4BDKHJNUK4SQMZHN243RQ=",
			},
			genlayermodelwrappers: {
				hash: "test"
			}
		},
		onnx_models: {
			all_MiniLM_L6_v2: {
				hash: "SVZQR7SKPVELTM4HOM74CYMJH3RQL5CJNNNTUWJGVS37NKQNUL2AKXDDKEMZRK76JP2WMHV3EHCJT2OWT7OZMTNGRIIW6WP3HWGCJZI=",
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
