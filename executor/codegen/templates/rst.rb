#!/usr/bin/env ruby

require 'pathname'
require 'json'
require 'ostruct'

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

def enumerate_trie_paths(entries, prefix = "")
	result = []
	entries.each { |entry|
		next if entry.nil?
		head = entry["head"]
		tail = entry["tail"]
		current = prefix.empty? ? head : "#{prefix}:#{head}"
		if tail.is_a?(String) && tail.start_with?('$')
			result << [current, tail[1..]]
		elsif tail.is_a?(Array)
			non_null = tail.compact
			is_terminal = (tail.length != non_null.length) || non_null.empty?
			result << [current, nil] if is_terminal
			result.concat(enumerate_trie_paths(non_null, current))
		end
	}
	result
end

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

	when "consts"
		buf << '.. _gvm-def-consts-' << t_os.name.gsub('_', '-') << ":\n\n"
		buf << t_os.name << "\n"
		buf << '-' * t_os.name.size << "\n\n"

		buf << "Type: " << t_os.repr << "\n\n"

		t_os.values.each { |k, v|
			buf << '.. _gvm-def-consts-value-' << t_os.name.gsub('_', '-') << '-' << k.gsub('_', '-') << ":\n\n"
			buf << k << "\n"
			buf << '~' * k.size << "\n\n"
			buf << "Value: ``" << v.to_s << "``\n\n"
		}

	when "str_trie"
		entries = t_os.values.map { |e| unfold_trie_entry(e) }
		buf << '.. _gvm-def-str-trie-' << t_os.name.gsub('_', '-') << ":\n\n"
		buf << t_os.name << "\n"
		buf << '-' * t_os.name.size << "\n\n"

		buf << "Type: str_trie\n\n"

		enumerate_trie_paths(entries).each { |path, param|
			rst_name = path.gsub('_', '-').gsub(':', '-')
			buf << '.. _gvm-def-str-trie-value-' << t_os.name.gsub('_', '-') << '-' << rst_name << ":\n\n"
			buf << "``#{path}``\n"
			buf << '~' * (path.size + 4) << "\n\n"
			if param
				buf << "Param: #{param}\n\n"
			end
		}

	else
		raise "unknown type #{t_os.type}"
	end
}

File.write(Pathname.new(out_path), buf)
