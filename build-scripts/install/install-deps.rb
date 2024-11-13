#!/usr/bin/env ruby

# frozen_string_literal: true

STDERR.sync = true
STDOUT.sync = true

require 'open3'
require 'pathname'
require 'logger'
require 'rubygems/package'
require 'zlib'
require 'net/http'
require 'shellwords'

require 'optparse'

options = {
	runners: false
}

OptionParser.new do |opts|
	opts.on '--genvm'
	opts.on '--rust'
	opts.on '--cross-linux-aarch64'
	opts.on '--zig'
	opts.on '--rust-det'
	opts.on '--os'
	opts.on '--wasi'
	opts.on '--test'
end.parse!(into: options)

logger = Logger.new(STDOUT, level: Logger::DEBUG)
logger.formatter = proc do |severity, datetime, progname, msg|
	#date_format = datetime.strftime("%H:%M:%S")
	if severity == "ERROR"
		severity = "\e[31m#{severity}\e[0m"
	end
	"#{severity.ljust(5)} #{msg}\n"
end
$logger = logger

def run_command(*cmd, chdir: nil)
	$logger.info("running #{cmd} at #{chdir || Dir.pwd}")
	buf = String.new
	kws = {}
	if not chdir.nil?
		kws[:chdir] = chdir
	end
	Open3.popen2e(*cmd.map { |s| s.to_s }, **kws) { |stdin, stdout, wait_thr|
		stdin.close()
		stdout.each_line { |l|
			puts "\t#{l}"
			buf << l << "\n"
		}
		exit_status = wait_thr.value
		if exit_status != 0
			raise "command #{cmd.map{ |x| Shellwords.escape x }.join(' ')} failed"
		end
	}
	buf
end

def find_executable(name)
	paths = ENV['PATH'].split(':')
	paths << '/usr/bin'
	paths << '/bin'
	paths << "#{ENV['HOME']}/.local/bin"
	paths << "#{ENV['HOME']}/.cargo/bin"
	paths.each { |p|
		check = ['', '.elf', '.exe']
		check.each { |c|
			cur_p = Pathname.new(p).join("#{name}#{c}")
			if cur_p.exist?()
				$logger.debug("located #{name} at #{cur_p}")
				return cur_p
			end
		}
	}
	return nil
end

$bash = find_executable 'bash'

TARGET_TRIPLE = Proc.new do
	o, e, s = Open3.capture3('rustc --version --verbose')
	raise "rustc failed #{o} #{e}" if not s.success?
	res = o.match(/host: ([a-zA-Z0-9_\-]*)/)[1]
	res
rescue
	RUBY_PLATFORM
end.call()

logger.info("detected target is #{TARGET_TRIPLE}")
$logger = logger

OS = (Proc.new {
	re = {
		'linux' => /linux/i,
		'macos' => /darwin|macos|apple/i,
		'windows' => /windows/i,
	}
	re.each { |k, v|
		if v =~ TARGET_TRIPLE
			break k
		end
	}
}).call()

PLATFORM = (Proc.new {
	re = {
		'amd64' => /x86_64|amd64/i,
		'aarch64' => /aarch64|arm64/i,
	}
	re.each { |k, v|
		if v =~ TARGET_TRIPLE
			break k
		end
	}
}).call()

logger.info("detected OS is #{OS}")
logger.info("detected PLATFORM is #{PLATFORM}")

root = Pathname.new(__FILE__).realpath.parent
while not root.join('.genvm-monorepo-root').exist?()
	root = root.parent
end
logger.debug("genvm root is #{root}")

download_dir = root.join('tools', 'downloaded')
download_dir.mkpath()
$download_dir = download_dir

logger.debug("download dir is #{download_dir}")

def load_packages_from_lists(dir)
	$logger.info("downloading #{dir} packages")
	case OS
	when 'linux'
		if Pathname.new('/etc/lsb-release').exist?()
			run_command $bash, Pathname.new(__FILE__).parent.join('src', dir, 'ubuntu.sh')
		else
			$logger.error("auto install of packages for linux excluding ubuntu is not supported")
		end
	when 'macos'
		run_command $bash, Pathname.new(__FILE__).parent.join('src', dir, 'brew.sh')
	else
		$logger.error("auto install of packages for your os is not supported")
	end
end

if options[:os]
	load_packages_from_lists 'os'
	#if options[:'cross-linux-aarch64']
	#	load_packages_from_lists 'os-linux-aarch64'
	#end
end

if not RUBY_VERSION =~ /^3\./
	logger.error("ruby must be at least 3.0, yours is #{RUBY_VERSION}")
end

if options[:rust] || options[:'rust-det']
	rustup = find_executable('rustup')
	if rustup.nil?
		logger.debug("downloading rust")
		puts `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --component rust-fmt`
		rustup = find_executable 'rustup'
	else
		logger.debug("rustup is already installed at #{rustup}")
	end
	raise "rustup not found" if rustup.nil?
end

if options[:'cross-linux-aarch64']
	# ZIG
	# FIXME(kp2pml30) check platform
	require_relative './src/webget.rb'
	download_to = download_dir.join('zig.tar.xz')
	read_file(
		uri: URI("https://ziglang.org/builds/zig-linux-x86_64-0.14.0-dev.2238+1db8cade5.tar.xz"),
		path: download_to
	)
	extract_tar(download_dir.join('zig'), download_to)

	# openssl
	download_to = download_dir.join('openssl-aarch.tar.xz')
	read_file(
		uri: URI("http://mirror.archlinuxarm.org/aarch64/core/openssl-1.0-1.0.2.u-1-aarch64.pkg.tar.xz"),
		path: download_to,
		use_ssl: false,
	)
	extract_tar(download_dir.join('cross-aarch64-libs'), download_to, trim_first: false)
end

if options[:rust]
	puts `cd "#{root}" && #{rustup} show active-toolchain || #{rustup} toolchain install`
	run_command(rustup, 'component', 'add', 'rustfmt', chdir: root)
end

if options[:'rust-det']
	ext_path = root.join('runners', 'cpython-and-ext', 'extension')
	cur_toolchain = run_command(rustup, 'show', 'active-toolchain', chdir: ext_path)
	cur_toolchain = cur_toolchain.lines.map { |l| l.strip }.filter { |l| l.size != 0 }.last
	cur_toolchain = /^([a-zA-Z0-9\-_]+)/.match(cur_toolchain)[1]
	logger.debug("installing for toolchain #{cur_toolchain}")
	run_command(rustup, 'target', 'add', '--toolchain', cur_toolchain, 'wasm32-wasip1', chdir: ext_path)
	run_command(rustup, 'component', 'add', '--toolchain', cur_toolchain, 'rust-std', chdir: ext_path)
end

if options[:rust] and options[:'cross-linux-aarch64']
	ext_path = root.join('executor')
	cur_toolchain = run_command(rustup, 'show', 'active-toolchain', chdir: ext_path)
	cur_toolchain = cur_toolchain.lines.map { |l| l.strip }.filter { |l| l.size != 0 }.last
	cur_toolchain = /^([a-zA-Z0-9\-_]+)/.match(cur_toolchain)[1]
	logger.debug("installing for toolchain #{cur_toolchain}")
	run_command(rustup, 'target', 'add', '--toolchain', cur_toolchain, 'aarch64-unknown-linux-gnu', chdir: ext_path)
	run_command(rustup, 'component', 'add', '--toolchain', cur_toolchain, 'rust-std', chdir: ext_path)
end

if options[:wasi]
	logger.debug("downloading runners dependencies")
	require_relative './src/wasi-sdk.rb'

	if find_executable('docker').nil?
		logger.error("docker is required")
	end
end

if options[:genvm]
	logger.debug("downloading genvm dependencies")
end

if options[:test]
	load_packages_from_lists 'test'
end
