require 'pathname'

src_dir, dst_file = ARGV

buf = String.new

src_dir = Pathname.new(src_dir)

src_dir.glob('**/*.txt').sort.each do |file|
	name = file.relative_path_from(src_dir).to_s
	buf << "### #{name}\n\n"
	buf << file.read
	buf << "\n\n"
end

Pathname.new(dst_file).dirname.mkpath
File.write(dst_file, buf)
