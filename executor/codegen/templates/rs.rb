#!/usr/bin/env ruby
require 'erb'
require 'pathname'
require 'json'
require 'ostruct'

def to_camel(s)
	s.split('_').map { |x| if x.size() == 0 then x else x[0].upcase + x[1..].downcase end }.join('')
end

def rust_repr(s)
	if s == "str"
		"&'static str"
	else
		s
	end
end

def rust_repr_from(s)
	if s == "str"
		"&str"
	else
		s
	end
end

def dump(s)
	s.kind_of?(String) ? s.dump : s.to_s
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

def rust_param_type(param)
	case param
	when "str" then "&str"
	else param
	end
end

def gen_rust_trie_inner(entries, struct_name, prefix_parts, buf, root_name, self_terminal_parts = nil)
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
				gen_rust_trie_inner(non_null, child_struct_name, current_parts, buf, root_name, child_terminal)
			end
		end
	}

	unless struct_name.empty?
		buf << "pub struct #{struct_name};\n\n"
		buf << "impl #{struct_name} {\n"
		if self_terminal_parts
			str_val = self_terminal_parts.join(" ")
			buf << "    pub const fn val(&self) -> #{root_name} { #{root_name}(Cow::Borrowed(\"#{str_val}\")) }\n"
		end
		leaves.each { |name, parts|
			str_val = parts.join(" ")
			buf << "    pub const fn #{name.downcase}(&self) -> #{root_name} { #{root_name}(Cow::Borrowed(\"#{str_val}\")) }\n"
		}
		methods.each { |name, child_struct|
			buf << "    pub const fn #{name.downcase}(&self) -> #{child_struct} { #{child_struct} }\n"
		}
		buf << "}\n\n"
	end

	param_nodes.each { |cs_name, param, parts|
		fmt_str = parts.join(" ") + " {v}"
		buf << "pub struct #{cs_name};\n\n"
		buf << "impl #{cs_name} {\n"
		buf << "    pub fn val_#{param}(&self, v: #{rust_param_type param}) -> #{root_name} {\n"
		buf << "        #{root_name}(Cow::Owned(format!(\"#{fmt_str}\")))\n"
		buf << "    }\n"
		buf << "}\n\n"
	}

	[leaves, methods]
end

def gen_rust_trie_builder(entries, root_name, buf)
	mod_buf = String.new
	leaves, methods = gen_rust_trie_inner(entries, "", [], mod_buf, root_name)

	mod_name = "__#{root_name}"

	unless mod_buf.empty?
		buf << "#[allow(non_snake_case)]\n"
		buf << "#[rustfmt::skip]\n"
		buf << "pub mod #{mod_name} {\n"
		buf << "    use std::borrow::Cow;\n"
		buf << "    use super::#{root_name};\n\n"
		mod_buf.each_line { |line|
			buf << (line.strip.empty? ? "\n" : "    #{line}")
		}
		buf << "}\n\n"
	end

	buf << "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\n"
	buf << "pub struct #{root_name}(pub Cow<'static, str>);\n\n"
	buf << "impl Into<String> for #{root_name} {\n"
	buf << "    fn into(self) -> String {\n"
	buf << "        self.0.into()\n"
	buf << "    }\n"
	buf << "}\n"
	buf << "#[rustfmt::skip]\n"
	buf << "impl #{root_name} {\n"
	leaves.each { |name, parts|
		str_val = parts.join(" ")
		buf << "    pub const fn #{name.downcase}() -> Self { Self(Cow::Borrowed(\"#{str_val}\")) }\n"
	}
	methods.each { |name, child_struct|
		buf << "    pub const fn #{name.downcase}() -> #{mod_name}::#{child_struct} { #{mod_name}::#{child_struct} }\n"
	}
	buf << "}\n\n"
end

# editorconfig-checker-disable
ENUM_TEMPLATE_STR = <<-EOF
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
% if repr != "str"
#[repr(<%= repr %>)]
pub enum <%= to_camel name %> {
% values.each { |k, v|
    <%= to_camel k %> = <%= dump v %>,
% }
}
% else
pub enum <%= to_camel name %> {
% values.each { |k, v|
    <%= to_camel k %>,
% }
}
% end

impl <%= to_camel name %> {
% if repr != "str" && values.values.all? { |v| v.is_a?(Integer) } && values.values.sort == (0...values.length).to_a
    pub const SIZE: usize = <%= values.length %>;
% end
    pub fn value(self) -> <%= rust_repr repr %> {
        match self {
%   values.each { |k, v|
            <%= to_camel name %>::<%= to_camel k %> => <%= dump v %>,
%   }
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
% values.each { |k, v|
            <%= to_camel name %>::<%= to_camel k %> => "<%= k %>",
% }
        }
    }
}

impl TryFrom<<%= rust_repr_from repr %>> for <%= to_camel name %> {
    type Error = ();

    fn try_from(value: <%= rust_repr_from repr %>) -> Result<Self, ()> {
        match value {
% values.each { |k, v|
            <%= dump v %> => Ok(<%= to_camel name %>::<%= to_camel k %>),
% }
            _ => Err(()),
        }
    }
}
EOF
# editorconfig-checker-enable

ENUM_TEMPLATE = ERB.new(ENUM_TEMPLATE_STR, trim_mode: "%")

json_path, out_path = ARGV

json_data = JSON.load_file(Pathname.new(json_path))
has_str_trie = json_data.any? { |t| t["type"] == "str_trie" }

buf = String.new

buf << "// This file is auto-generated. Do not edit!\n\n"
buf << "#![allow(dead_code, clippy::redundant_static_lifetimes)]\n";
buf << "\n"
buf << "use serde::{Deserialize, Serialize};\n\n"

if has_str_trie
	buf << "use std::borrow::Cow;\n\n"
end

json_data.each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
	when "const"
		buf << "pub const #{t_os.name.upcase}: #{rust_repr t_os.repr} = #{dump t_os.value};\n"
	when "consts"
		buf << "pub mod #{t_os.name} {\n"
		t_os.values.each { |k, v|
			buf << "    pub const #{k.upcase}: #{rust_repr t_os.repr} = #{dump v};\n"
		}
		buf << "}\n\n"
	when "str_trie"
		entries = t_os.values.map { |e| unfold_trie_entry(e) }
		gen_rust_trie_builder(entries, to_camel(t_os.name), buf)
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
