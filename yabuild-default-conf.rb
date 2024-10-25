require 'open3'

conf = {
	profile: "debug",
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
			hash: "ZUI5E6TYSPFORPORNJWUEM4GIVBLY3TISIO4QMDDTCPGOOD5YWQ3NMD72KEUXRI42OVE3VKMJTNSACIJ5AD4E5KYHB7VBBGGHXLDL3Q=",
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
}.to_ostruct

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

if not conf.tools.clang.nil?
	begin
		run_command_success conf.tools.clang, '-c', '-o', root_conf.join('a.o'), root_src.join('build-scripts', 'test-tools', 'clang-mold', 'a.c')
		run_command_success conf.tools.clang, '-c', '-o', root_conf.join('b.o'), root_src.join('build-scripts', 'test-tools', 'clang-mold', 'b.c')
	rescue => e
		logger.warn("clang doesn't work #{conf.tools.clang} #{e}")
		conf.tools.clang = nil
	else
		logger.info("clang works")
	end
end
if not conf.tools.clang.nil? and not conf.tools.mold.nil?
	begin
		run_command_success conf.tools.clang, "-fuse-ld=#{conf.tools.mold}", '-o', root_conf.join('ab'), root_conf.join('a.o'), root_conf.join('b.o')
		run_command_success root_conf.join('ab')
	rescue => e
		logger.warn("mold doesn't work #{conf.tools.mold} #{e}")
		conf.tools.mold = nil
	else
		logger.info("mold works")
	end
end

conf
