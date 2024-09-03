class CargoBuildTarget < Target
	attr_reader :output_file
	def initialize(dir, name, target, release)
		cargo_out_dir = dir.join('target')
		if not target.nil?
			cargo_out_dir = cargo_out_dir.join(target)
		end
		@release = release
		if release
			cargo_out_dir = cargo_out_dir.join('release')
		else
			cargo_out_dir = cargo_out_dir.join('debug')
		end
		@cargo_out_dir = cargo_out_dir
		@dir = dir
		@is_lib = name == "lib"
		if @is_lib
			# avoid toml dependency
			File.read(@dir.join('Cargo.toml')).lines.each { |l|
				m = l.match(/name\s*=\s*"(.*)"/)
				if not m.nil?
					@name = m[1]
					break
				end
			}
			@name = 'lib' + @name.gsub('-', '_')
			suff = NATIVE_LIB_EXT
		else
			@name = name
			suff = ""
		end
		@output_file = @cargo_out_dir.join(@name + suff)
		super(@output_file, [])
	end

	protected def dump_rules_impl(buf)
		buf << "  WD = #{Shellwords.escape @dir}\n"
		if @is_lib
			buf << "  FLAGS = --lib"
		else
			buf << "  FLAGS = --bin #{@name}"
		end
		if @release
			buf << " --release"
		end
		buf << "\n"
		buf << "  depfile = #{@cargo_out_dir.join(@name)}.d\n"
	end

	def mode
		"CARGO_BUILD"
	end
end

class CargoCopyTarget < Target
	def initialize(to, from, parent)
		super(to, [from])
		@parent = parent
	end

	protected def dump_rules_impl(buf)
	end

	def mode
		"COPY"
	end

	def add_deps(*deps)
		@parent.add_deps(*deps)
	end
end

add_rule(<<-EOF
rule CARGO_BUILD
  command = cd $WD && cargo build $FLAGS
  pool = console
  description = $DESC

EOF
)

self.define_singleton_method(:target_cargo_build) do |out_file:, dir: nil, name:, target: nil, release: false, &blk|
	if dir.nil?
		dir = cur_src
	end

	trg = CargoBuildTarget.new(dir, name, target, release)
	trg_copy = CargoCopyTarget.new(out_file, trg.output_file, trg)

	@targets.push(trg)
	@targets.push(trg_copy)
	return_target(trg_copy, &blk)
	trg_copy
end
