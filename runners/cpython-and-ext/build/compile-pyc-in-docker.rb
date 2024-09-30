#!/usr/bin/env ruby

require 'open3'
require 'pathname'

log_file, id_file, map_dir = ARGV

id = File.read(id_file).strip

File.open(log_file, 'wt') { |f|
	command = ['docker', 'run', '--network=none', '--rm', '-v', "#{map_dir}:/compile-py", '--entrypoint', '/scripts-py/compile.sh', id, '/compile-py']
	puts "run: #{command}"
	Open3.popen2e(*command) { |stdin, stdout, wait_thr|
		stdin.close()
		stdout.each_line { |l|
			puts l
			f.write(l)
		}
		exit_status = wait_thr.value
		if exit_status != 0
			raise "docker failed"
		end
	}
}
