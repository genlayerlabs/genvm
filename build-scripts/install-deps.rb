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
require 'mkmf'

require 'optparse'

options = {
	runners: false
}

OptionParser.new do |opts|
	opts.on '--genvm'
	opts.on '--rust'
	opts.on '--os'
	opts.on '--wasi'
end.parse!(into: options)

logger = Logger.new(STDOUT, level: Logger::DEBUG)
logger.formatter = proc do |severity, datetime, progname, msg|
	#date_format = datetime.strftime("%H:%M:%S")
	if severity == "ERROR"
		severity = "\e[31m#{severity}\e[0m"
	end
	"#{severity.ljust(5)} #{msg}\n"
end

TARGET_TRIPLE = Proc.new do
	o, e, s = Open3.capture3('rustc --version --verbose')
	raise "rustc failed #{o} #{e}" if not s.success?
	res = o.match(/host: ([a-zA-Z0-9_\-]*)/)[1]
	res
rescue
	RUBY_PLATFORM
end.call()

logger.info("detected target is #{TARGET_TRIPLE}")

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

logger.debug("download dir is #{download_dir}")

if options[:os]
	logger.info("downloading OS packages")
	case OS
	when 'linux'
		if Pathname.new('/etc/lsb-release').exist?()
			`/usr/bin/bash "#{Pathname.new(__FILE__).parent.join('src', 'ubuntu.sh')}"`
		else
			logger.error("auto install of packages for linux excluding ubuntu is not supported")
		end
	when 'macos'
		`sh "#{Pathname.new(__FILE__).parent.join('src', 'brew.sh')}"`
	else
		logger.error("auto install of packages for your os is not supported")
	end
end

if not RUBY_VERSION =~ /^3\./
	logger.error("ruby must be at least 3.0, yours is #{RUBY_VERSION}")
end

if options[:rust]
	rustup = find_executable('rustup')
	if rustup.nil?
		logger.debug("downloading rust")
		`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --component rust-fmt`
		rustup = ENV['HOME'] + "/.cargo/bin/rustup"
	else
		logger.debug("rustup is already installed at #{rustup}")
	end
	`cd "#{root}" && #{rustup} show active-toolchain || #{rustup} toolchain install`

	cur_toolchain = `#{rustup} show active-toolchain`
	cur_toolchain = cur_toolchain.strip
	cur_toolchain = /^([a-zA-Z0-9\-_]+)/.match(cur_toolchain)[1]
	logger.debug("installing for toolchain #{cur_toolchain}")
	`cd "#{root}" && #{rustup} target add --toolchain #{cur_toolchain} wasm32-wasip1`
end

if options[:wasi]
	logger.debug("downloading runners dependencies")
	src = Pathname.new(__FILE__).parent.join('src', 'wasi-sdk.rb').read
	eval(src, binding)

	if find_executable('docker').nil?
		logger.error("docker is required")
	end
end

if options[:genvm]
	logger.debug("downloading genvm dependencies")
end
