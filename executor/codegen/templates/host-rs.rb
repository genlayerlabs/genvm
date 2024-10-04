#!/usr/bin/env ruby
require 'erb'
require 'pathname'
require 'json'

def to_camel(s)
  s.split('_').map { |x| if x.size() == 0 then x else x[0].upcase + x[1..].downcase end }.join('')
end

ENUM_TEMPLATE_STR = <<-EOF
#[derive(PartialEq)]
#[repr(<%= size %>)]
pub enum <%= to_camel name %> {
% values.each { |k, v|
    <%= to_camel k %> = <%= v %>,
% }
}
EOF

ENUM_TEMPLATE = ERB.new(ENUM_TEMPLATE_STR, trim_mode: "%")

buf = String.new

JSON.load_file(Pathname.new(__dir__).parent.join('data', 'host-fns.json')).each { |t|
  t_os = OpenStruct.new(t)
  case t_os.type
  when "enum"
    buf << ENUM_TEMPLATE.result(t_os.instance_eval { binding })
  else
    raise "unknown type #{t_os.type}"
  end
}

File.write(Pathname.new(__dir__).parent.parent.join('src', 'host', 'host_fns.rs'), buf)
