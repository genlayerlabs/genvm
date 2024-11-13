wasi_dir = $download_dir.join('wasi-sdk-24')

require_relative './webget.rb'

if wasi_dir.exist?()
	$logger.info("wasi-sdk-24 already exists")
else
	download_to = $download_dir.join('wasi-sdk-24.tar.gz')
	if not download_to.exist?
		$logger.info("downloading wasi-sdk-24 to #{download_to}")
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
	extract_tar(wasi_dir, download_to)
end
