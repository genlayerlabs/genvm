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

def gen_py_trie_node(entry, indent)
	head = entry["head"]
	tail = entry["tail"]
	if tail.is_a?(String) && tail.start_with?('$')
		param_type = tail[1..]
		"#{indent}StrTrieNode(head=#{dump head}, is_terminal=False, param=#{dump param_type}, children=())"
	elsif tail.is_a?(Array)
		non_null = tail.compact
		is_terminal = (tail.length != non_null.length) || non_null.empty?
		py_terminal = is_terminal ? "True" : "False"
		if non_null.empty?
			"#{indent}StrTrieNode(head=#{dump head}, is_terminal=True, param=None, children=())"
		else
			children_code = non_null.map { |c| gen_py_trie_node(c, indent + "\t") }.join(",\n")
			"#{indent}StrTrieNode(head=#{dump head}, is_terminal=#{py_terminal}, param=None, children=(\n#{children_code},\n#{indent}))"
		end
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

json_path, out_path = ARGV

json_data = JSON.load_file(Pathname.new(json_path))
has_str_trie = json_data.any? { |t| t["type"] == "str_trie" }

buf = String.new

buf << <<-EOF
# This file is auto-generated. Do not edit!

from enum import IntEnum, StrEnum
import typing
EOF

if has_str_trie
	buf << "\n\nclass StrTrieNode(typing.NamedTuple):\n"
	buf << "\thead: str\n"
	buf << "\tis_terminal: bool\n"
	buf << "\tparam: typing.Optional[str]\n"
	buf << "\tchildren: tuple\n"
end

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
		nodes_code = entries.map { |e| gen_py_trie_node(e, "\t") }.join(",\n")
		buf << "\n\n#{t_os.name.upcase}: typing.Final[tuple[StrTrieNode, ...]] = (\n#{nodes_code},\n)\n"
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
