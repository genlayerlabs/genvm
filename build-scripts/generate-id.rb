require 'pathname'

root, out = ARGV
out = Pathname.new(out)
tag=`cd "#{root}" && git describe --abbrev=40 --tags --dirty`
tag = tag.strip
puts "detected tag is `#{tag}`"
if not out.exist? or tag != out.read().strip
	out.write(tag)
end
