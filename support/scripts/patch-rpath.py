#!/usr/bin/env python3

import argparse
import logging
import os
import subprocess
import sys
from pathlib import Path

import lief

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

lief.logging.set_level(lief.logging.LEVEL.ERROR)


def _resolve_needed(binary_path: Path, libraries, search_dir: Path | None):
	"""Replace absolute needed paths with basenames and compute rpaths from search_dir."""
	replacements: dict[str, str] = {}
	resolved_rpaths: set[str] = set()

	for lib in libraries:
		name = lib if isinstance(lib, str) else lib.name
		if '/' not in name:
			continue
		basename = name.rsplit('/', 1)[-1]
		replacements[name] = basename

		if search_dir is not None:
			found = list(search_dir.rglob(basename))
			if found:
				lib_dir = found[0].parent
				rel = os.path.relpath(lib_dir, binary_path.parent)
				resolved_rpaths.add(f'$ORIGIN/{rel}' if rel != '.' else '$ORIGIN')
				logger.info(f'Resolved {basename} -> {found[0]} (rpath: $ORIGIN/{rel})')

	return replacements, resolved_rpaths


def patch_elf(
	binary: lief.ELF.Binary, rpaths: list[str], binary_path: Path, search_dir: Path | None
) -> None:
	logger.info('Processing ELF binary')

	replacements, resolved_rpaths = _resolve_needed(
		binary_path, list(binary.libraries), search_dir
	)
	for entry in binary.dynamic_entries:
		if entry.tag == lief.ELF.DynamicEntry.TAG.NEEDED and entry.name in replacements:
			old_name = entry.name
			entry.name = replacements[old_name]
			logger.info(f'Replaced needed: "{old_name}" -> "{entry.name}"')

	all_rpaths = list(dict.fromkeys(rpaths + sorted(resolved_rpaths)))

	if binary.has(lief.ELF.DynamicEntry.TAG.RPATH):
		rpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RPATH)
		old_rpath = str(rpath_entry.value)
		rpath_entry.paths = all_rpaths
		logger.info(f'Updated RPATH from "{old_rpath}" to: "{rpath_entry.value}"')
	else:
		rpath_entry = lief.ELF.DynamicEntryRpath(all_rpaths)
		binary.add(rpath_entry)
		logger.info(f'Added new RPATH entry: "{all_rpaths}"')


def patch_macho(
	binary: lief.MachO.Binary,
	rpaths: list[str],
	binary_path: Path,
	search_dir: Path | None,
) -> None:
	logger.info('Processing Mach-O binary')

	dylib_cmds = [
		cmd
		for cmd in binary.commands
		if cmd.command
		in (
			lief.MachO.LoadCommand.TYPE.LOAD_DYLIB,
			lief.MachO.LoadCommand.TYPE.LOAD_WEAK_DYLIB,
		)
	]

	_, resolved_rpaths = _resolve_needed(
		binary_path,
		[cmd.name for cmd in dylib_cmds],
		search_dir,
	)

	known_libs: dict[str, str] = {}
	if search_dir is not None:
		for p in search_dir.rglob('*'):
			if p.is_file() and p.suffix in ('.dylib', '.so'):
				known_libs[p.name] = p.name

	# /usr/local/lib/libiconv.2.dylib -> libiconv.dylib (versioned name differs from shipped name)
	if 'libiconv.dylib' in known_libs:
		known_libs['libiconv.2.dylib'] = 'libiconv.dylib'

	for cmd in dylib_cmds:
		if cmd.name.startswith('@'):
			continue
		basename = cmd.name.rsplit('/', 1)[-1] if '/' in cmd.name else cmd.name
		target_name = known_libs.get(basename)
		if target_name is None:
			continue
		old_name = cmd.name
		cmd.name = '@rpath/' + target_name
		logger.info(f'Replaced library reference: "{old_name}" -> "{cmd.name}"')

	all_rpaths = list(dict.fromkeys(rpaths + sorted(resolved_rpaths)))
	for rpath in all_rpaths:
		macho_rpath = rpath.replace('$ORIGIN', '@loader_path')
		rpath_cmd = lief.MachO.RPathCommand.create(macho_rpath)
		binary.add(rpath_cmd)
		logger.info(f'Added RPATH to Mach-O binary: "{macho_rpath}"')


def patch_binary(
	path: Path, rpaths: list[str], codesign: bool, search_dir: Path | None = None
) -> None:
	logger.info(f'Patching {path} with rpaths {rpaths}')

	binary = lief.parse(str(path))
	if not binary:
		raise RuntimeError(f'Failed to parse binary at {path}')

	if binary.format == lief.Binary.FORMATS.ELF:
		patch_elf(binary, rpaths, path, search_dir)
	elif binary.format == lief.Binary.FORMATS.MACHO:
		patch_macho(binary, rpaths, path, search_dir)
	else:
		raise RuntimeError(f'Unsupported binary format for {path}: {binary.format}')

	binary.write(str(path))
	logger.info(f'Successfully patched binary: {path}')

	if binary.format == lief.Binary.FORMATS.ELF:
		needed = [lib if isinstance(lib, str) else lib.name for lib in binary.libraries]
	else:
		needed = [
			cmd.name
			for cmd in binary.commands
			if cmd.command
			in (
				lief.MachO.LoadCommand.TYPE.LOAD_DYLIB,
				lief.MachO.LoadCommand.TYPE.LOAD_WEAK_DYLIB,
			)
		]
	logger.info(f'Needed libraries after patching: {needed}')

	if codesign and binary.format == lief.Binary.FORMATS.MACHO:
		logger.info(f'Code signing Mach-O binary: {path}')
		subprocess.run(['rcodesign', 'sign', str(path)], check=True)


def main() -> int:
	parser = argparse.ArgumentParser(
		description='Set rpath on an ELF or Mach-O binary. $ORIGIN in rpaths is translated to @loader_path for Mach-O.'
	)
	parser.add_argument(
		'--rpath',
		action='append',
		default=[],
		required=True,
		help='Rpath entry (may be repeated). Use $ORIGIN-relative paths.',
	)
	parser.add_argument(
		'--codesign',
		action='store_true',
		help='Ad-hoc code sign Mach-O binaries after patching.',
	)
	parser.add_argument(
		'--search-dir',
		default=None,
		help='Directory to search for needed libraries to compute rpaths.',
	)
	parser.add_argument('binary', help='Path to binary to patch')

	args = parser.parse_args()

	search_dir = Path(args.search_dir) if args.search_dir else None
	patch_binary(Path(args.binary), args.rpath, args.codesign, search_dir)
	return 0


if __name__ == '__main__':
	sys.exit(main())
