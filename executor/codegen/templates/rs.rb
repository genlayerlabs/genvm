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

def dump(s)
	s.kind_of?(String) ? s.dump : s.to_s
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

impl TryFrom<<%= rust_repr repr %>> for <%= to_camel name %> {
    type Error = ();

    fn try_from(value: <%= rust_repr repr %>) -> Result<Self, ()> {
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

buf = String.new

buf << "#![allow(dead_code, clippy::redundant_static_lifetimes)]\n\n";
buf << "use serde_derive::{Deserialize, Serialize};\n\n"

JSON.load_file(Pathname.new(json_path)).each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
	when "const"
		buf << "pub const #{t_os.name.upcase}: #{rust_repr t_os.repr} = #{dump t_os.value};\n"
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
