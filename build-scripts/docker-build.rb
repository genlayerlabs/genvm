#!/usr/bin/env ruby

require 'open3'

img = nil

log_file, id_file, path, dfile = ARGV

if dfile.nil?
	dfile = 'Dockerfile'
end

if path.nil?
	path = '.'
end

begin
	Pathname.new(id_file).unlink()
rescue
end

File.open(log_file, 'wt') { |f|
	puts `docker buildx ls`
	command = ['docker', 'buildx', 'build', '--network=host', '--progress=plain', '-f', dfile, path]
	puts "run: #{command}"
	Open3.popen2e(*command) { |stdin, stdout, wait_thr|
		stdin.close()
		stdout.each_line { |l|
			puts l
			f.write(l)
			mtch = /writing image (sha256:[0-9a-z]+)/.match(l)
			if mtch
				if not img.nil? and img != mtch[1]
					raise "found two write logs: #{img} vs #{mtch[1]}"
				end
				img = mtch[1]
			end
		}
		exit_status = wait_thr.value
		if exit_status != 0
			raise "docker failed"
		end
	}
	puts "detected image is #{img}"
	raise "Image not found" if img.nil?
	File.write(id_file, img)
}
