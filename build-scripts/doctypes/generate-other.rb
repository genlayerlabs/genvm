require 'pathname'
require 'erb'

from, to = ARGV

from = Pathname.new from
to = Pathname.new to

children = from.glob("**/*.py")
children.map! { |c| c.relative_path_from(from).to_s }
children.map! { |c| c.gsub(/(\/__init__)?\.py$/, '').gsub(/\//, '.') }
has_already = ['genlayer', 'genlayer.std', 'genlayer.std.advanced', 'genlayer.py.calldata', 'genlayer.py.eth']
children.filter! { |c| not has_already.include?(c) }
children.sort!

to.mkpath

children.each { |c|
to.join("#{c}.rst").write(<<-EOF
========#{'=' * c.size}
Package #{c}
========#{'=' * c.size}

.. warning::
   This is an internal module

.. automodule:: #{c}

EOF
)
}

template = <<-EOF
GenLayer SDK Internal packages
==============================

.. warning::
   This are internal modules, they are subject to change between version. This page is provided only for reference.

   For that reason users should not use anything form these packages directly, but use re-exports

.. toctree::
   :maxdepth: 2
   :caption: Contents:

% children.each { |c|
   <%= c %>
% }

EOF

TEMPLATE = ERB.new(template, trim_mode: "%")

to.join('index.rst').write(TEMPLATE.result)
