require 'open3'

CONFIGURATOR = self

class CargoBuildTarget < Target
	attr_reader :output_file
	def initialize(dir, name, target, profile, features, flags, env, out_file)
		@env = env
		@flags = flags
		@features = features
		@out_file = out_file
		@target_dir = CONFIGURATOR.root_build.join('generated', 'rust-target') # dir.join('target')
		# @target_dir = dir.join('target')
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
		@cargo_output_file = @cargo_out_dir.join(@name + suff)
		@output_file = if @out_file.nil? then @cargo_output_file else @out_file end
		super(outputs: [@output_file], inputs: [dir.join('Cargo.toml')], rule: 'CARGO_BUILD')
	end

	def dump_vars(cb)
		cb.('CWD', Shellwords.escape(@dir))
		if @env.size > 0
			buf = String.new
			buf << 'env'
			@env.each { |k, v|
				buf << ' ' << k << '=' << Shellwords.escape(v).gsub(/\\=/, '=')
			}
			cb.('ENV', buf)
		end

		if not @out_file.nil?
			cb.("CARGO_MB_COPY", "&& cp #{@cargo_output_file} #{@output_file}")
		end

		flags = []

		if @is_lib
			flags << '--lib'
		else
			flags << '--bin' << @name
		end
		flags << '--target-dir' << @target_dir
		if @profile != "debug"
			flags << '--profile' << @profile
		end
		if @target
			flags << "--target" << @target
		end
		if @features.size > 0
			flags << '--features' << @features.join(',')
		end
		flags_val = String.new
		DefaultTargets::escape_args_to flags_val, flags
		DefaultTargets::escape_args_to flags_val, @flags
		cb.('FLAGS', flags_val)
		cb.('depfile', "#{@cargo_out_dir.join(@name)}.d")
	end
end

# editorconfig-checker-disable
ninja_files_parts['genvm'] << NinjaPieceRaw.new(<<-EOF
rule CARGO_BUILD
  command = cd $CWD && $ENV cargo build $FLAGS $CARGO_MB_COPY && touch -c $out
  pool = console

EOF
)
# editorconfig-checker-enable
ninja_files_parts[''] << NinjaPieceRaw.new('include genvm.ninja')

self.define_singleton_method(:target_cargo_build) do |out_file: nil, dir: nil, name:, target: nil, profile: "debug", features: [], flags: [], env: {}, **kwargs, &blk|
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

	trg = CargoBuildTarget.new(dir, name, target, profile, features, flags, env, out_file)
	return_target(trg, **kwargs, &blk)
end
