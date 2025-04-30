#!/usr/bin/env ruby
require 'erb'
require 'pathname'
require 'json'
require 'ostruct'

def to_camel(s)
	s.split('_').map { |x| if x.size() == 0 then x else x[0].upcase + x[1..].downcase end }.join('')
end

# editorconfig-checker-disable
ENUM_TEMPLATE_STR = <<-EOF
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(<%= size %>)]
pub enum <%= to_camel name %> {
% values.each { |k, v|
    <%= to_camel k %> = <%= v %>,
% }
}

#[allow(dead_code)]
impl <%= to_camel name %> {
    pub fn str_snake_case(self) -> &'static str {
        match self {
% values.each { |k, v|
            <%= to_camel name %>::<%= to_camel k %> => "<%= k %>",
% }
        }
    }
}

impl TryFrom<<%= size %>> for <%= to_camel name %> {
    type Error = ();

    fn try_from(value: <%= size %>) -> Result<Self, ()> {
        match value {
% values.each { |k, v|
            <%= v %> => Ok(<%= to_camel name %>::<%= to_camel k %>),
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

buf << "use serde_derive::{Serialize, Deserialize};\n"

JSON.load_file(Pathname.new(json_path)).each { |t|
	t_os = OpenStruct.new(t)
	case t_os.type
	when "enum"
		buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)

# Pathname.new(__dir__).parent.join('data', 'host-fns.json')
# Pathname.new(__dir__).parent.parent.join('src', 'host', 'host_fns.rs')
