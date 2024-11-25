require 'pathname'

root, out = ARGV
out = Pathname.new(out).realdirpath
Dir.chdir root

require 'openssl'

diff = `git diff -U0 -- executor/src`
diff = diff.encode(Encoding::UTF_8)
diff = OpenSSL::Digest.new('SHA3-224').digest(diff)
diff = diff.bytes.pack("c*").unpack("H*").first
diff = diff[...16]

tag=`git describe --abbrev=16 --tags --dirty="-dirty_#{diff}"`
tag = tag.strip
puts "detected tag is `#{tag}`"
if not out.exist? or tag != out.read().strip
	out.write(tag)
end
