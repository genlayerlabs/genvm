def read_file(uri:, path:, use_ssl: true)
	if path.exist?
		$logger.info("already exists #{path}, skipping download")
		return
	end
	$logger.info("downloading #{uri} => #{path}")
	loop {
		request = Net::HTTP::Get.new(uri)
		Net::HTTP.start(uri.host, uri.port, :use_ssl => use_ssl) do |http|
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

def extract_tar(base_dir, path, trim_first: true)
	if base_dir.exist?
		$logger.info("already exists #{base_dir}, skipping extraction")
		return
	end
	base_dir.mkpath
	if path.to_s.end_with?('.xz')
		require 'rubygems/package'
		begin
			Gem::Specification::find_by_name('ruby-xz')
		rescue Gem::LoadError
			require 'rubygems/commands/install_command'
			cmd = Gem::Commands::InstallCommand.new
			cmd.handle_options ['--user-install', 'ruby-xz']
			begin
				cmd.execute
			rescue Gem::SystemExitException
			end
		end
		require 'xz'
		decomp = path.sub_ext('.tar')
		XZ::decompress_file(path, decomp)
		tar_extract = Gem::Package::TarReader.new(File.open(decomp))
	else
		tar_extract = Gem::Package::TarReader.new(Zlib::GzipReader.open(path))
	end
	begin
		tar_extract.rewind
		dest = nil
		g_name = Proc.new { |v|
			names = v.split('/')
			if trim_first
				base_dir.join(*names[1...names.size])
			else
				base_dir.join(*names)
			end
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
