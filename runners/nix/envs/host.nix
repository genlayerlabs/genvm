#{  pkgs
#, ...
#}:
#{
#	imports = [
#		./wasm.nix
#	];
#
#	packages = with pkgs; [
#		zig
#		bash
#		vim
#		(pkgs.writeShellScriptBin "clang" ''
#		${zig.outPath}/bin/zig cc "$@"
#		'')
#		(pkgs.writeShellScriptBin "clang++" ''
#		${zig.outPath}/bin/zig c++ "$@"
#		'')
#	];
#
#	env = {
#		CC = "clang";
#		CXX = "clang++";
#	};
#}
