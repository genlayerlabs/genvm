#!/usr/bin/env ruby
require 'erb'
require 'pathname'
require 'json'
require 'ostruct'

def to_camel(s)
	s.split('_').map { |x| if x.size() == 0 then x else x[0].upcase + x[1..].downcase end }.join('')
end

def dump(s)
	s.kind_of?(String) ? "'#{s}'" : s.to_s
end

def py_repr(s)
	if s =~ /^(u|i)\d+$/
		"int"
	else
		s
	end
end

def unfold_trie_entry(entry)
	if entry.nil?
		nil
	elsif entry.is_a?(String)
		{"head" => entry, "tail" => []}
	elsif entry.is_a?(Hash)
		tail = entry["tail"]
		if tail.is_a?(Array)
			{"head" => entry["head"], "tail" => tail.map { |e| unfold_trie_entry(e) }}
		else
			entry
		end
	end
end

def gen_py_trie_inner(entries, struct_name, prefix_parts, buf, root_name, self_terminal_parts = nil)
	leaves = []
	methods = []
	param_nodes = []

	entries.each { |entry|
		next if entry.nil?
		head = entry["head"]
		tail = entry["tail"]
		current_parts = prefix_parts + [head]
		child_struct_name = struct_name + to_camel(head)

		if tail.is_a?(String) && tail.start_with?('$')
			param_type = tail[1..]
			methods << [head, child_struct_name]
			param_nodes << [child_struct_name, param_type, current_parts]
		elsif tail.is_a?(Array)
			non_null = tail.compact
			is_terminal = (tail.length != non_null.length) || non_null.empty?

			if non_null.empty?
				leaves << [head, current_parts]
			else
				methods << [head, child_struct_name]
				child_terminal = is_terminal ? current_parts : nil
				gen_py_trie_inner(non_null, child_struct_name, current_parts, buf, root_name, child_terminal)
			end
		end
	}

	unless struct_name.empty?
		buf << "class _#{root_name}#{struct_name}:\n"
		if self_terminal_parts
			str_val = self_terminal_parts.join(" ")
			buf << "\t@staticmethod\n"
			buf << "\tdef val() -> '#{root_name}':\n"
			buf << "\t\treturn #{root_name}('#{str_val}')\n"
		end
		leaves.each { |name, parts|
			str_val = parts.join(" ")
			buf << "\t@staticmethod\n"
			buf << "\tdef #{name.downcase}() -> '#{root_name}':\n"
			buf << "\t\treturn #{root_name}('#{str_val}')\n"
		}
		methods.each { |name, child_struct|
			buf << "\t@staticmethod\n"
			buf << "\tdef #{name.downcase}() -> '_#{root_name}#{child_struct}':\n"
			buf << "\t\treturn _#{root_name}#{child_struct}()\n"
		}
		buf << "\n"
	end

	param_nodes.each { |cs_name, param, parts|
		fmt_str = parts.join(" ")
		buf << "class _#{root_name}#{cs_name}:\n"
		buf << "\t@staticmethod\n"
		buf << "\tdef val_#{param}(v: #{py_repr param}) -> '#{root_name}':\n"
		buf << "\t\treturn #{root_name}(f'#{fmt_str} {v}')\n"
		buf << "\n"
	}

	[leaves, methods]
end

def gen_py_trie_builder(entries, root_name, buf)
	inner_buf = String.new
	leaves, methods = gen_py_trie_inner(entries, "", [], inner_buf, root_name)

	buf << inner_buf

	buf << "class #{root_name}:\n"
	buf << "\t__slots__ = ('value',)\n"
	buf << "\tdef __init__(self, value: str):\n"
	buf << "\t\tself.value = value\n"
	buf << "\tdef __str__(self) -> str:\n"
	buf << "\t\treturn self.value\n"
	leaves.each { |name, parts|
		str_val = parts.join(" ")
		buf << "\t@staticmethod\n"
		buf << "\tdef #{name.downcase}() -> '#{root_name}':\n"
		buf << "\t\treturn #{root_name}('#{str_val}')\n"
	}
	methods.each { |name, child_struct|
		buf << "\t@staticmethod\n"
		buf << "\tdef #{name.downcase}() -> '_#{root_name}#{child_struct}':\n"
		buf << "\t\treturn _#{root_name}#{child_struct}()\n"
	}
	buf << "\n"
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

json_path, out_path = ARGV

json_data = JSON.load_file(Pathname.new(json_path))

buf = String.new

buf << <<-EOF
# This file is auto-generated. Do not edit!

from enum import IntEnum, StrEnum
import typing
EOF

json_data.each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
	when "const"
		buf << "\n\n#{t_os.name.upcase}: typing.Final[#{py_repr t_os.repr}] = #{dump t_os.value}\n"
	when "consts"
		buf << "\n\nclass _#{to_camel t_os.name}(typing.NamedTuple):\n"
		t_os.values.each { |k, v|
			buf << "\t#{k.upcase}: #{py_repr t_os.repr} = #{dump v}\n"
		}
		buf << "\n#{t_os.name}: typing.Final = _#{to_camel t_os.name}()\n"
	when "str_trie"
		entries = t_os.values.map { |e| unfold_trie_entry(e) }
		buf << "\n"
		gen_py_trie_builder(entries, to_camel(t_os.name), buf)
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
