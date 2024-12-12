require 'pathname'
require 'erb'

from, to = ARGV

from = Pathname.new from
to = Pathname.new to

children = from.glob("**/*.py")
children.map! { |c| c.relative_path_from(from).to_s }
children.map! { |c| c.gsub(/(\/__init__)?\.py$/, '').gsub(/\//, '.') }
children.filter! { |c| c != 'genlayer' and c != 'genlayer.std' }
children.sort!

template = <<-EOF

Internal packages
=================

.. warning::
	Users shouldn't use anything from this package directly, use re-exports

% children.each { |c|
<%= '=' * c.size %>
<%= c %>
<%= '=' * c.size %>

.. automodule:: <%= c %>

% }

EOF

TEMPLATE = ERB.new(template, trim_mode: "%")

to.write(TEMPLATE.result)
