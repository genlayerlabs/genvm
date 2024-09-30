#!/usr/bin/env ruby

require 'open3'
require 'pathname'

require 'optparse'

options = {

}
OptionParser.new do |parser|
	parser.on('--cp FILE') do |cp_file|
		cp_file = Pathname.new(cp_file)
		File.write(options[:out_dir].join(cp_file.basename), File.read(cp_file))
	end
	parser.on('--id-file FILE')
	parser.on('--log FILE')
	parser.on('--out-dir DIR') do |v|
		puts "got out dir #{v}"
		out_dir = Pathname.new(v)
		options[:out_dir] = out_dir
		out_dir.mkpath()
	end
	parser.on('--entrypoint PATH')
end.parse!(into: options)

log_file = options[:log]
out_dir = options[:out_dir]

id = File.read(options[:'id-file']).strip

File.open(log_file, 'wt') { |f|
	command = ['docker', 'run', '--network=none', '--rm', '-v', "#{out_dir}:/out", '--entrypoint', options[:entrypoint], id, *ARGV]
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
