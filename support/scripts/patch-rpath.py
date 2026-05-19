#!/usr/bin/env python3

import argparse
import logging
import subprocess
import sys
from pathlib import Path

import lief

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

lief.logging.set_level(lief.logging.LEVEL.ERROR)


def patch_elf(binary: lief.ELF.Binary, rpaths: list[str]) -> None:
	logger.info('Processing ELF binary')

	if binary.has(lief.ELF.DynamicEntry.TAG.RPATH):
		rpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RPATH)
		old_rpath = str(rpath_entry.value)
		rpath_entry.paths = rpaths
		logger.info(f'Updated RPATH from "{old_rpath}" to: "{rpath_entry.value}"')
	else:
		rpath_entry = lief.ELF.DynamicEntryRpath(rpaths)
		binary.add(rpath_entry)
		logger.info(f'Added new RPATH entry: "{rpaths}"')


def patch_macho(binary: lief.MachO.Binary, rpaths: list[str]) -> None:
	logger.info('Processing Mach-O binary')

	for cmd in binary.commands:
		if cmd.command in (
			lief.MachO.LoadCommand.TYPE.LOAD_DYLIB,
			lief.MachO.LoadCommand.TYPE.LOAD_WEAK_DYLIB,
		):
			if cmd.name == '/usr/local/lib/libiconv.2.dylib':
				old_name = cmd.name
				cmd.name = '@rpath/libiconv.dylib'
				logger.info(f'Replaced library reference: "{old_name}" -> "{cmd.name}"')
			elif '/' not in cmd.name:
				old_name = cmd.name
				cmd.name = '@rpath/' + cmd.name
				logger.info(f'Replaced library reference: "{old_name}" -> "{cmd.name}"')

	for rpath in rpaths:
		macho_rpath = rpath.replace('$ORIGIN', '@loader_path')
		rpath_cmd = lief.MachO.RPathCommand.create(macho_rpath)
		binary.add(rpath_cmd)
		logger.info(f'Added RPATH to Mach-O binary: "{macho_rpath}"')


def patch_binary(path: Path, rpaths: list[str], codesign: bool) -> None:
	logger.info(f'Patching {path} with rpaths {rpaths}')

	binary = lief.parse(str(path))
	if not binary:
		raise RuntimeError(f'Failed to parse binary at {path}')

	if binary.format == lief.Binary.FORMATS.ELF:
		patch_elf(binary, rpaths)
	elif binary.format == lief.Binary.FORMATS.MACHO:
		patch_macho(binary, rpaths)
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
	parser.add_argument('binary', help='Path to binary to patch')

	args = parser.parse_args()

	patch_binary(Path(args.binary), args.rpath, args.codesign)
	return 0


if __name__ == '__main__':
	sys.exit(main())
