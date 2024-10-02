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
