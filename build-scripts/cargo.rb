require 'open3'

CONFIGURATOR = self

class CargoBuildTarget < Target
	attr_reader :output_file
	def initialize(dir, name, target, profile, features, flags)
		@flags = flags
		@features = features
		# @target_dir = CONFIGURATOR.root_build.join('generated', 'rust-target') # dir.join('target')
		@target_dir = dir.join('target')
		cargo_out_dir = @target_dir
		@target = target
		if not target.nil?
			cargo_out_dir = cargo_out_dir.join(target)
		end
		@profile = profile
		cargo_out_dir = cargo_out_dir.join(profile)
		@cargo_out_dir = cargo_out_dir
		@dir = dir
		@is_lib = name =~ /^(static|dy)lib$/
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
			if name == 'staticlib'
				suff = NATIVE_STATIC_LIB_EXT
			else
				suff = NATIVE_SHARED_LIB_EXT
			end
		else
			@name = name
			if @target =~ /wasm/
				suff = ".wasm"
			else
				suff = ""
			end
		end
		@output_file = @cargo_out_dir.join(@name + suff)
		super(@output_file, [dir.join('Cargo.toml')])
	end

	protected def dump_rules_impl(buf)
		buf << "  WD = #{Shellwords.escape @dir}\n"
		if @is_lib
			buf << "  FLAGS = --lib"
		else
			buf << "  FLAGS = --bin #{@name}"
		end
		buf << " --target-dir " << @target_dir.to_s
		if @profile != "debug"
			buf << " --profile=#{@profile}"
		end
		if @target
			buf << " --target #{@target}"
		end
		if @features.size > 0
			buf << " --features #{@features.join(',')}"
		end
		escape_args_to buf, @flags
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
  command = cd $WD && cargo build $FLAGS && touch $out
  pool = console
  description = $DESC

EOF
)

self.define_singleton_method(:target_cargo_build) do |out_file: nil, dir: nil, name:, target: nil, profile: "debug", features: [], flags: [], **kwargs, &blk|
	if target.nil?
		@dflt_target ||= Proc.new {
			o, e, s = Open3.capture3('rustc --version --verbose')
			raise "rustc failed #{o} #{e}" if not s.success?
			res = o.match(/host: ([a-zA-Z0-9_\-]*)/)[1]
			@logger.info("default rust target is set to #{res}")
			res
		}.call()
		target = @dflt_target
	end
	if dir.nil?
		dir = cur_src
	end

	trg = CargoBuildTarget.new(dir, name, target, profile, features, flags)

	if out_file.nil?
		return return_target(trg, **kwargs, &blk)
	end

	register_target(trg)

	trg_copy = CargoCopyTarget.new(out_file, trg.output_file, trg)
	return_target(trg_copy, **kwargs, &blk)
end
