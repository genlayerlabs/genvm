require 'net/http'

download_dir = root_src.join('tools', 'downloaded')
expected_path = download_dir.join('zig', 'zig')
if not expected_path.exist?
	logger.info 'downloading zig'
	$logger = logger
	require_relative './src/webget.rb'
	download_dir.mkpath
	download_to = download_dir.join('zig.tar.xz')
	read_file(
		uri: URI("https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz"),
		path: download_to
	)
	extract_tar(download_dir.join('zig'), download_to)
end

raise 'could not install zig' if not expected_path.exist?

$cross_cc = root_src.join('build-scripts', 'zig-driver.py')
