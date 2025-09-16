#!/usr/bin/env ruby

require 'pathname'
require 'shellwords'
require 'optparse'
require 'json'

class Pathname
	def is_subpath_of?(other)
		self_expanded = self.expand_path
		other_expanded = Pathname.new(other).expand_path

		begin
			relative = self_expanded.relative_path_from(other_expanded)
			!relative.to_s.start_with?('..')
		rescue ArgumentError
			false  # Different filesystem roots
		end
	end
end

$source_dir = Pathname.new(__FILE__).dirname.expand_path

module Ninja
	class RawStr < String
		def initialize(str)
			super(str)
			freeze
		end
	end

	AND = RawStr.new('&&')
	VAR_IN = RawStr.new('$in')
	VAR_OUT = RawStr.new('$out')

	module Private
		def self.dump_var_value_str(file, value)
			if value.kind_of? Pathname
				if value.relative?
					value = $source_dir.join(value).relative_path_from(file.build_dir).to_s
				elsif value.is_subpath_of?(file.build_dir) or value.is_subpath_of?($source_dir)
					value = value.relative_path_from(file.build_dir).to_s
				else
					value = value.to_s
				end
			end

			if value.kind_of? RawStr
				file.buf << value
			elsif value.kind_of? String
				file.buf << Shellwords.escape(value).gsub(/\\=/, '=')
			elsif value.kind_of? Symbol
				file.buf << Shellwords.escape(value.to_s).gsub(/\\=/, '=')
			else
				raise "Unsupported variable type: #{value.class}"
			end
		end

		def self.dump_var_value(file, value)
			if value.kind_of? Array
				value.each_with_index do |v, i|
					if i > 0
						file.buf << ' '
					end
					dump_var_value(file, v)
				end
			else
				dump_var_value_str(file, value)
			end
		end
	end

	class BaseBuilder
		def var(name, value)
			@properties[name] = value
		end

		def validate
			raise "Abstract method called"
		end
	end

	class RuleBuilder < BaseBuilder
		def initialize(name)
			@name = name
			@properties = {}
		end

		def command(*cmd)
			@properties[:command] = cmd
		end

		def description(desc)
			@properties[:description] = desc
		end

		def validate
			raise "Rule #{@name} must have a command" unless @properties[:command]
		end

		def finish(file)
			file.buf << "rule #{@name}\n"
			@properties.each do |key, value|
				file.buf << "  #{key} = "
				Private::dump_var_value(file, value)
				file.buf << "\n"
			end
			file.buf << "\n"
		end
	end

	class BuildBuilder < BaseBuilder
		def initialize(rule, outputs)
			@rule = rule
			@outputs = Array.new(outputs)
			@implicit_outputs = []
			@dependencies = []
			@implicit_dependencies = []
			@order_only_dependencies = []
			@properties = {}
		end

		private def _add_to(arr, item)
			if item.kind_of? Array
				arr.concat(item)
			else
				arr << item
			end
		end

		def add_output(output)
			_add_to(@outputs, output)
		end

		def add_implicit_output(output)
			_add_to(@implicit_outputs, output)
		end

		def add_dependency(dep)
			_add_to(@dependencies, dep)
		end

		def add_implicit_dependency(dep)
			_add_to(@implicit_dependencies, dep)
		end

		def add_order_only_dependency(dep)
			_add_to(@order_only_dependencies, dep)
		end

		def description(desc)
			@properties[:description] = desc
		end

		def validate
			if @outputs.empty?
				raise "Build rule must have at least one output"
			end
		end

		def finish(file)
			file.buf << "build"
			@outputs.each do |output|
				file.buf << ' '
				file.push_str output
			end
			if @implicit_outputs.size > 0
				file.buf << ' |'
				@implicit_outputs.each do |output|
					file.buf << ' '
					file.push_str output
				end
			end
			file.buf << ": "
			file.buf << @rule.to_s
			@dependencies.each do |dep|
				file.buf << ' '
				file.push_str dep
			end
			if @implicit_dependencies.size > 0
				file.buf << ' |'
				@implicit_dependencies.each do |dep|
					file.buf << ' '
					file.push_str dep
				end
			end

			if @order_only_dependencies.size > 0
				file.buf << ' ||'
				@order_only_dependencies.each do |dep|
					file.buf << ' '
					file.push_str dep
				end
			end

			file.buf << "\n"

			@properties.each do |key, value|
				file.buf << "  #{key} = "
				Private::dump_var_value(file, value)
				file.buf << "\n"
			end
			file.buf << "\n"
		end
	end

	class File
		attr_reader :buf, :build_dir

		def initialize(build_dir)
			@buf = String.new
			@build_dir = Pathname.new(build_dir).expand_path
			@build_dir.mkpath
		end

		def push_str(str)
			Private::dump_var_value_str(self, str)
		end

		def comment(text)
			text.each_line do |line|
				@buf << "# #{line.strip}\n"
			end
		end

		def var(name, value)
			@buf << "#{name} = "
			Private::dump_var_value(self, value)
			@buf << "\n\n"
		end

		def rule(name, &block)
			builder = RuleBuilder.new(name)
			builder.instance_eval(&block) if block_given?

			builder.validate
			builder.finish(self)
		end

		def build(rule, *outputs, &block)
			builder = BuildBuilder.new(rule, outputs)


			builder.instance_eval(&block) if block_given?

			builder.validate
			builder.finish(self)
		end
	end
end

def detect_rust_target
	rustc_output = `rustc -vV`
	rust_target = nil
	rustc_output.each_line do |line|
		if line.start_with?('host: ')
			rust_target = line[6..-1].strip
			break
		end
	end

	if rust_target == nil
		raise "Failed to detect rust target from 'rustc -vV' output"
	end

	rust_target
end

$rust_target = detect_rust_target()

$build_dir = Pathname.new('build')
$build_dir.mkpath

generator = Ninja::File.new $build_dir
generator.comment("Generated by configure.rb, DO NOT EDIT MANUALLY")

generator.var('ninja_required_version', '1.5')

generator.rule(:CLEAN) do
	command('ninja', Ninja::RawStr.new('$FILE_ARG'), '-t', 'clean', Ninja::RawStr.new('$TARGETS'))
	description 'Cleaning all built files...'
end

generator.rule(:HELP) do
	command('ninja', Ninja::RawStr.new('$FILE_ARG'), '-t', 'targets', 'rule', 'phony', 'rule', 'CLEAN', 'rule', 'HELP')
	description 'All primary targets available'
end

generator.var 'build_dir', $build_dir.to_s

generator.build(:CLEAN, 'clean')
generator.build(:HELP, 'help')

generator.rule(:phony_touch) do
	command 'touch', Ninja::VAR_OUT
end

generator.rule(:CUSTOM_COMMAND) do
	command 'cd', Ninja::RawStr.new('$CWD'), Ninja::AND, Ninja::RawStr.new('$ENV'), Ninja::RawStr.new('$COMMAND')
	description 'Running custom command'
end

generator.rule(:cp) do
	command 'cp', Ninja::VAR_IN, Ninja::VAR_OUT
end

generator.build(:CUSTOM_COMMAND, 'build.ninja') do
	add_dependency(Pathname.new(__FILE__))
	var(:COMMAND, [RbConfig.ruby, Pathname.new(__FILE__).expand_path.to_s] + ARGV)
	var :CWD, $source_dir.expand_path.to_s
end

$rust_target_dir = $build_dir.join('ya-build', 'rust-target').expand_path
$rust_target_dir.mkpath

generator.rule(:codegen) do
	command RbConfig.ruby, Ninja::VAR_IN, Ninja::VAR_OUT
end

generator.build(:codegen, $source_dir.join('executor', 'src', 'public_abi.rs')) do
	add_dependency($source_dir.join('executor', 'codegen', 'templates', 'rs.rb'))
	add_dependency($source_dir.join('executor', 'codegen', 'data', 'public-abi.json'))
end

generator.build(:codegen, $source_dir.join('runners', 'genlayer-py-std', 'src', 'genlayer', 'py', 'public_abi.py')) do
	add_dependency($source_dir.join('executor', 'codegen', 'templates', 'py.rb'))
	add_dependency($source_dir.join('executor', 'codegen', 'data', 'public-abi.json'))
end

generator.build(:codegen, $source_dir.join('tests', 'runner', 'host_fns.py')) do
	add_dependency($source_dir.join('executor', 'codegen', 'templates', 'py.rb'))
	add_dependency($source_dir.join('executor', 'codegen', 'data', 'host-fns.json'))
end

generator.build(:codegen, $source_dir.join('executor', 'src', 'host', 'host_fns.rs')) do
	add_dependency($source_dir.join('executor', 'codegen', 'templates', 'rs.rb'))
	add_dependency($source_dir.join('executor', 'codegen', 'data', 'host-fns.json'))
end

generator.build(:codegen, $source_dir.join('doc', 'website', 'src', 'spec', 'appendix', 'constants.rst')) do
	add_dependency($source_dir.join('executor', 'codegen', 'templates', 'rst.rb'))
	add_dependency($source_dir.join('executor', 'codegen', 'data', 'public-abi.json'))
end

generator.build(:phony, 'codegen') do
	add_dependency $source_dir.join('executor', 'src', 'public_abi.rs')
	add_dependency $source_dir.join('runners', 'genlayer-py-std', 'src', 'genlayer', 'py', 'public_abi.py')
	add_dependency $source_dir.join('tests', 'runner', 'host_fns.py')
	add_dependency $source_dir.join('executor', 'src', 'host', 'host_fns.rs')
	add_dependency $source_dir.join('doc', 'website', 'src', 'spec', 'appendix', 'constants.rst')
end

generator.rule(:cargo) do
	command \
		'cd', Ninja::RawStr.new('$wd'),
		Ninja::AND, Ninja::RawStr.new('$env'), 'cargo', Ninja::RawStr.new('$subcommand'), '--target', $rust_target, '--target-dir', $rust_target_dir.to_s, Ninja::RawStr.new('$extra_args'),
		Ninja::AND, 'cd', $build_dir.expand_path.to_s,
		Ninja::AND, 'touch', Ninja::VAR_OUT
	description 'Running cargo $subcommand'

	var :pool, :console
end

generator.rule(:cargo_build) do
	command \
		'cd', Ninja::RawStr.new('$wd'),
		Ninja::AND, Ninja::RawStr.new('$env'), 'cargo', 'build', '--target', $rust_target, '--target-dir', $rust_target_dir.to_s, Ninja::RawStr.new('$extra_args')

	description 'Running cargo $subcommand'

	var :depfile, Ninja::RawStr.new('$out.d')

	var :pool, :console
end

$info = {
	coverage_dir: $build_dir.join('cov').expand_path.to_s,
	build_dir: $build_dir.expand_path.to_s,
	rust_target_dir: $rust_target_dir.expand_path.to_s,
}

Pathname.new($info[:coverage_dir]).mkpath

$all_format = []
$all_clippy = []
$all_clippy_fix = []

def generator.register_cargo(rel_path, extra_args: [], build_to: nil)
	to = $build_dir.join('ya-build', *rel_path.split('/'))
	to.mkpath

	dir = Pathname.new(rel_path)
	all_files = dir.glob('**/*.rs') + [dir.join('Cargo.toml'), dir.join('Cargo.lock')]

	files_trg = to.join('files.trg')

	build(:phony_touch, files_trg) do
		add_implicit_dependency all_files
	end

	build(:cargo, to.join('clippy.trg')) do
		add_dependency files_trg
		var :subcommand, 'clippy'
		var :wd, dir
		var :extra_args, extra_args + ['--', '-A', 'clippy::upper_case_acronyms', '-Dwarnings']
	end

	build(:phony, 'target/' + rel_path + '/clippy') do
		add_dependency to.join('clippy.trg')
		description 'Run cargo clippy for ' + rel_path
	end
	$all_clippy.push(to.join('clippy.trg'))

	build(:cargo, to.join('clippy.fix.trg')) do
		add_dependency files_trg
		var :subcommand, 'clippy'
		var :wd, dir
		var :extra_args, extra_args + ['--fix', '--allow-dirty', '--allow-staged', '--', '-A', 'clippy::upper_case_acronyms', '-Dwarnings']
	end
	$all_clippy_fix.push(to.join('clippy.fix.trg'))

	build(:CUSTOM_COMMAND, 'target/' + rel_path + '/fmt') do
		add_dependency files_trg

		var :command, [
			'cd', dir,
			Ninja::AND, 'cargo', 'fmt',
		]

		description 'Run cargo fmt for ' + rel_path
	end

	$all_format.push('target/' + rel_path + '/fmt')

	if build_to != nil
		bin_name = $rust_target_dir.join($rust_target, 'debug', build_to.split('/').last).expand_path.to_s
		build(:cargo_build, bin_name) do
			add_dependency files_trg
			var :wd, dir
			var :extra_args, extra_args
			var :env, Ninja::RawStr.new('RUSTFLAGS="-C instrument-coverage" LLVM_PROFILE_FILE=/dev/null')
		end

		build(:cp, build_to) do
			add_dependency bin_name
		end
	end
end

generator.register_cargo('executor', build_to: 'out/executor/vTEST/bin/genvm')
generator.register_cargo('executor/common')
generator.register_cargo('modules/implementation', extra_args: ['--features', 'vendored-lua'], build_to: 'out/bin/genvm-modules')
generator.register_cargo('modules/interfaces')

generator.rule(:nix_eval) do
	command [
		Ninja::RawStr.new('WD=$$(pwd)'), Ninja::AND,
		'cd', Ninja::RawStr.new('$wd'), Ninja::AND,
		'nix', 'eval', '--verbose', '--impure', '--read-only', '--show-trace', '--json', '--expr', Ninja::RawStr.new('$expr'),
		Ninja::RawStr.new('>'), Ninja::RawStr.new('$$WD/$out'),
	]

	var :pool, :console
end

generator.build(:nix_eval, 'out/executor/vTEST/data/latest.json') do
	var 'expr', 'let drv = import ./runners ; in builtins.listToAttrs (builtins.map (x: { name = x.id; value = builtins.convertHash { hash = x.hash; toHashFormat = "nix32"; }; }) drv)'
	var 'wd', $source_dir
end

generator.build(:nix_eval, 'out/executor/vTEST/data/all.json') do
	var 'expr', 'let drv = import ./runners ; in builtins.listToAttrs (builtins.map (x: { name = x.id; value = [ (builtins.convertHash { hash = x.hash; toHashFormat = "nix32"; }) ]; }) drv)'
	var 'wd', $source_dir
end

generator.build(:CUSTOM_COMMAND, 'target/runners') do
	var :command, [
		'nix', 'build', '-v', '-L', '-o', $build_dir.join('runners-nix'), '--file', $source_dir.join('runners', 'build-here.nix'),
		Ninja::AND, 'mkdir', '-p', './out/runners',
		Ninja::AND, 'cp', '-r', './runners-nix/.', './out/runners/.',
		Ninja::AND, 'chmod', '-R', '+w', './out/runners/.',
	]

	add_dependency $source_dir.join('runners', 'build-here.nix')

	add_implicit_dependency $source_dir.join('runners').glob('**/*').filter { |f|
		f.file? && !f.each_filename.any? { |x| ["test", "tests", "fuzz"].include? x }
	}

	var :pool, :console
end

generator.build(:phony, 'cargo/fmt') do
	add_dependency $all_format
end

generator.build(:phony, 'cargo/clippy') do
	add_dependency $all_clippy
end

generator.build(:phony, 'cargo/clippy/fix') do
	add_dependency $all_clippy_fix
end

generator.build(:phony, 'all/bin') do
	add_dependency 'out/executor/vTEST/bin/genvm'
	add_dependency 'out/bin/genvm-modules'

	doInstall = Proc.new { |from, to|
		install_dir = Pathname($source_dir).join(*from.split('/'))
		install_dir.glob('**/*').each do |f|
			next if f.directory?

			out = to + '/' + f.relative_path_from(install_dir).to_s
			add_dependency out
			generator.build(:cp, out) do
				add_dependency f
			end
		end
	}

	doInstall.('executor/install', 'out/executor/vTEST')
	doInstall.('modules/install', 'out')
end

generator.build(:phony, 'all') do
	add_dependency 'all/bin'

	add_dependency 'out/executor/vTEST/data/latest.json'
	add_dependency 'out/executor/vTEST/data/all.json'

	add_dependency 'target/runners'
end

generator.buf << "default all\n\n"

Pathname.new($build_dir.join('build.ninja')).write(generator.buf)
Pathname.new($build_dir.join('info.json')).write(JSON.pretty_generate($info))
