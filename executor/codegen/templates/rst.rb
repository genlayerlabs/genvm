#!/usr/bin/env ruby

require 'pathname'
require 'json'
require 'ostruct'

json_path, out_path = ARGV

buf = String.new

buf << <<-EOF
Constants
=========

EOF

JSON.load_file(Pathname.new(json_path)).each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << '.. _gvm-def-enum-' << t_os.name.gsub('_', '-') << ":\n\n"
		buf << t_os.name << "\n"
		buf << '-' * t_os.name.size << "\n\n"

		buf << "Type: " << t_os.repr << "\n\n"

		t_os.values.each { |k, v|
			buf << ".. _gvm-def-enum-value-" << t_os.name.gsub('_', '-') << '-' << k.gsub('_', '-') << ":\n\n"

			buf << k << "\n"
			buf << '~' * k.size << "\n\n"
			buf << "Value: ``" << v.to_s << "``\n\n"
		}

	when "const"
		buf << '.. _gvm-def-const-' << t_os.name.gsub('_', '-') << ":\n\n"
		buf << t_os.name << "\n"
		buf << '-' * t_os.name.size << "\n\n"

		buf << "Type: " << t_os.repr << "\n\n"
		buf << "Value: ``" << t_os.value.to_s << "``\n\n"
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
