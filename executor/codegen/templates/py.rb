#!/usr/bin/env ruby
require 'erb'
require 'pathname'
require 'json'
require 'ostruct'

def to_camel(s)
	s.split('_').map { |x| if x.size() == 0 then x else x[0].upcase + x[1..].downcase end }.join('')
end

def dump(s)
	s.kind_of?(String) ? s.dump : s.to_s
end

def py_repr(s)
	if s =~ /^(u|i)\d+$/
		"int"
	else
		s
	end
end

# editorconfig-checker-disable
ENUM_TEMPLATE_STR = <<-EOF


class <%= to_camel name %>(<%= repr == "str" ? "StrEnum" : "IntEnum" %>):
% values.each { |k, v|
	<%= k.upcase %> = <%= dump v %>
% }
EOF
# editorconfig-checker-enable

ENUM_TEMPLATE = ERB.new(ENUM_TEMPLATE_STR, trim_mode: "%")

buf = String.new

buf << <<-EOF
from enum import IntEnum, StrEnum
import typing
EOF

json_path, out_path = ARGV

JSON.load_file(Pathname.new(json_path)).each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
	when "const"
		buf << "\n\n#{t_os.name.upcase}: typing.Final[#{py_repr t_os.repr}] = #{dump t_os.value}\n"
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
