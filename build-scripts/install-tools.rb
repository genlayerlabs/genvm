#!/usr/bin/env ruby

# frozen_string_literal: true

if not RUBY_VERSION =~ /^3\./
	raise "ruby must be at least 3.0, yours is #{RUBY_VERSION}"
end

require 'open3'
require 'pathname'
require 'logger'
require 'rubygems/package'
require 'zlib'
require 'net/http'

logger = Logger.new(STDOUT, level: Logger::DEBUG)
logger.formatter = proc do |severity, datetime, progname, msg|
	#date_format = datetime.strftime("%H:%M:%S")
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

download_dir = root.join('third-party')
download_dir.mkpath()

logger.debug("download dir is #{download_dir}")

wasi_dir = download_dir.join('wasi-sdk-24')

def read_file(uri:, path:)
	loop {
		request = Net::HTTP::Get.new(uri)
		Net::HTTP.start(uri.host, uri.port, :use_ssl => true) do |http|
			http.request(request) { |response|
				case response
				when Net::HTTPRedirection
					uri = URI(response['location'])
				when Net::HTTPSuccess
					File.open(path, 'wb') { |file|
						response.read_body { |chunk|
							file.write chunk
						}
					}
					return
				else
					raise "invalid response #{response}"
				end
			}
		end
	}
end

if wasi_dir.exist?()
	logger.info("wasi-sdk-24 already exists")
else
	download_to = download_dir.join('wasi-sdk-24.tar.gz')
	if not download_to.exist?
		logger.info("downloading wasi-sdk-24 to #{download_to}")
		Net::HTTP.start("github.com") do |http|
			plat = case PLATFORM
				when 'amd64'
				'x86_64'
			when 'aarch64'
				'arm64'
			else
				raise "unsupported platform"
			end
			puts "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-#{plat}-#{OS}.tar.gz"
			read_file(
				uri: URI("https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-#{plat}-#{OS}.tar.gz"),
				path: download_to
			)
		end
	end
	tar_extract = Gem::Package::TarReader.new(Zlib::GzipReader.open(download_to))
	begin
		tar_extract.rewind
		dest = nil
		g_name = Proc.new { |v|
			names = v.split('/')
			wasi_dir.join(*names[1...names.size])
		}
		tar_extract.each do |entry|
			if entry.full_name == '././@LongLink'
				dest = g_name.call(entry.read.strip)
				next
			end
			dest ||= g_name.call(entry.full_name)
			if entry.directory?
				dest.mkpath
			elsif entry.file?
				File.open dest, "wb" do |f|
					f.write entry.read
				end
				FileUtils.chmod entry.header.mode, dest, :verbose => false
			elsif entry.header.typeflag == '2' #Symlink!
				File.symlink entry.header.linkname, dest
			end
			dest = nil
		end
	ensure
		tar_extract.close
	end
end
