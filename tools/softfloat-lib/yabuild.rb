project('softfloat') {

	c_files = [
		"berkeley-softfloat-3/source/f32_add.c",
		"berkeley-softfloat-3/source/f32_div.c",
		"berkeley-softfloat-3/source/f32_eq_signaling.c",
		"berkeley-softfloat-3/source/f32_eq.c",
		"berkeley-softfloat-3/source/f32_isSignalingNaN.c",
		"berkeley-softfloat-3/source/f32_le_quiet.c",
		"berkeley-softfloat-3/source/f32_le.c",
		"berkeley-softfloat-3/source/f32_lt_quiet.c",
		"berkeley-softfloat-3/source/f32_lt.c",
		"berkeley-softfloat-3/source/f32_mul.c",
		"berkeley-softfloat-3/source/f32_mulAdd.c",
		"berkeley-softfloat-3/source/f32_rem.c",
		"berkeley-softfloat-3/source/f32_roundToInt.c",
		"berkeley-softfloat-3/source/f32_sqrt.c",
		"berkeley-softfloat-3/source/f32_sub.c",
		"berkeley-softfloat-3/source/f32_to_f64.c",
		"berkeley-softfloat-3/source/f32_to_i32_r_minMag.c",
		"berkeley-softfloat-3/source/f32_to_i32.c",
		"berkeley-softfloat-3/source/f32_to_i64_r_minMag.c",
		"berkeley-softfloat-3/source/f32_to_i64.c",
		"berkeley-softfloat-3/source/f32_to_ui32_r_minMag.c",
		"berkeley-softfloat-3/source/f32_to_ui32.c",
		"berkeley-softfloat-3/source/f32_to_ui64_r_minMag.c",
		"berkeley-softfloat-3/source/f32_to_ui64.c",
		"berkeley-softfloat-3/source/f64_add.c",
		"berkeley-softfloat-3/source/f64_div.c",
		"berkeley-softfloat-3/source/f64_eq_signaling.c",
		"berkeley-softfloat-3/source/f64_eq.c",
		"berkeley-softfloat-3/source/f64_isSignalingNaN.c",
		"berkeley-softfloat-3/source/f64_le_quiet.c",
		"berkeley-softfloat-3/source/f64_le.c",
		"berkeley-softfloat-3/source/f64_lt_quiet.c",
		"berkeley-softfloat-3/source/f64_lt.c",
		"berkeley-softfloat-3/source/f64_mul.c",
		"berkeley-softfloat-3/source/f64_mulAdd.c",
		"berkeley-softfloat-3/source/f64_rem.c",
		"berkeley-softfloat-3/source/f64_roundToInt.c",
		"berkeley-softfloat-3/source/f64_sqrt.c",
		"berkeley-softfloat-3/source/f64_sub.c",
		"berkeley-softfloat-3/source/f64_to_f32.c",
		"berkeley-softfloat-3/source/f64_to_i32_r_minMag.c",
		"berkeley-softfloat-3/source/f64_to_i32.c",
		"berkeley-softfloat-3/source/f64_to_i64_r_minMag.c",
		"berkeley-softfloat-3/source/f64_to_i64.c",
		"berkeley-softfloat-3/source/f64_to_ui32_r_minMag.c",
		"berkeley-softfloat-3/source/f64_to_ui32.c",
		"berkeley-softfloat-3/source/f64_to_ui64_r_minMag.c",
		"berkeley-softfloat-3/source/f64_to_ui64.c",
		"berkeley-softfloat-3/source/i32_to_f32.c",
		"berkeley-softfloat-3/source/i32_to_f64.c",
		"berkeley-softfloat-3/source/i64_to_f32.c",
		"berkeley-softfloat-3/source/i64_to_f64.c",
		"berkeley-softfloat-3/source/s_add128.c",
		"berkeley-softfloat-3/source/s_add256M.c",
		"berkeley-softfloat-3/source/s_addMagsF32.c",
		"berkeley-softfloat-3/source/s_addMagsF64.c",
		"berkeley-softfloat-3/source/s_approxRecip_1Ks.c",
		"berkeley-softfloat-3/source/s_approxRecip32_1.c",
		"berkeley-softfloat-3/source/s_approxRecipSqrt_1Ks.c",
		"berkeley-softfloat-3/source/s_approxRecipSqrt32_1.c",
		"berkeley-softfloat-3/source/s_countLeadingZeros16.c",
		"berkeley-softfloat-3/source/s_countLeadingZeros32.c",
		"berkeley-softfloat-3/source/s_countLeadingZeros64.c",
		"berkeley-softfloat-3/source/s_countLeadingZeros8.c",
		"berkeley-softfloat-3/source/s_eq128.c",
		"berkeley-softfloat-3/source/s_le128.c",
		"berkeley-softfloat-3/source/s_lt128.c",
		"berkeley-softfloat-3/source/s_mul128By32.c",
		"berkeley-softfloat-3/source/s_mul128To256M.c",
		"berkeley-softfloat-3/source/s_mul64ByShifted32To128.c",
		"berkeley-softfloat-3/source/s_mul64To128.c",
		"berkeley-softfloat-3/source/s_mulAddF32.c",
		"berkeley-softfloat-3/source/s_mulAddF64.c",
		"berkeley-softfloat-3/source/s_normRoundPackToF32.c",
		"berkeley-softfloat-3/source/s_normRoundPackToF64.c",
		"berkeley-softfloat-3/source/s_normSubnormalF32Sig.c",
		"berkeley-softfloat-3/source/s_normSubnormalF64Sig.c",
		"berkeley-softfloat-3/source/s_roundPackToF32.c",
		"berkeley-softfloat-3/source/s_roundPackToF64.c",
		"berkeley-softfloat-3/source/s_roundToI32.c",
		"berkeley-softfloat-3/source/s_roundToI64.c",
		"berkeley-softfloat-3/source/s_roundToUI32.c",
		"berkeley-softfloat-3/source/s_roundToUI64.c",
		"berkeley-softfloat-3/source/s_shiftRightJam128.c",
		"berkeley-softfloat-3/source/s_shiftRightJam128Extra.c",
		"berkeley-softfloat-3/source/s_shiftRightJam256M.c",
		"berkeley-softfloat-3/source/s_shiftRightJam32.c",
		"berkeley-softfloat-3/source/s_shiftRightJam64.c",
		"berkeley-softfloat-3/source/s_shiftRightJam64Extra.c",
		"berkeley-softfloat-3/source/s_shortShiftLeft128.c",
		"berkeley-softfloat-3/source/s_shortShiftRight128.c",
		"berkeley-softfloat-3/source/s_shortShiftRightJam128.c",
		"berkeley-softfloat-3/source/s_shortShiftRightJam128Extra.c",
		"berkeley-softfloat-3/source/s_shortShiftRightJam64.c",
		"berkeley-softfloat-3/source/s_shortShiftRightJam64Extra.c",
		"berkeley-softfloat-3/source/s_sub128.c",
		"berkeley-softfloat-3/source/s_sub256M.c",
		"berkeley-softfloat-3/source/s_subMagsF32.c",
		"berkeley-softfloat-3/source/s_subMagsF64.c",
		"berkeley-softfloat-3/source/softfloat_state.c",
		"berkeley-softfloat-3/source/ui32_to_f32.c",
		"berkeley-softfloat-3/source/ui32_to_f64.c",
		"berkeley-softfloat-3/source/ui64_to_f32.c",
		"berkeley-softfloat-3/source/ui64_to_f64.c",
		"spec/more_funcs.c",
		"spec/s_propagateNaNF32UI.c",
		"spec/s_propagateNaNF64UI.c",
		"spec/softfloat_raiseFlags.c",
	]

	clang = Pathname.new(config.wasiSdk).join('bin', 'clang')
	sysroot = Pathname.new(config.wasiSdk).join('share', 'wasi-sysroot')

	c_file_targets = c_files.map { |cf|
		cf = Pathname.new(cf)
		target_c(
			output_file: cur_build.join(cf.sub_ext('.o').basename),
			mode: "compile",
			file: cur_src.join(cf),
			cc: clang,
			flags: [
				'--target=wasm32-wasi', "--sysroot=#{sysroot}",
				'-flto', '-O3',
				'-DINLINE_LEVEL=9', '-DSOFTFLOAT_FAST_INT64',
				"-I#{cur_src.join("spec")}",
				"-I#{cur_src.join("berkeley-softfloat-3", "source", "include")}"
			]
		)
	}

	raw = target_c(
		output_file: cur_build.join('softfloat.raw.wasm'),
		mode: "link",
		objs: c_file_targets,
		cc: clang,
		flags: [
			'--target=wasm32-wasi', "--sysroot=#{sysroot}",
			'-flto', '-O3',
			'-Wl,--no-entry,--export-all',
			'-static',
			'-lc'
		]
	)

	out = cur_build.join('softfloat.wasm')
	target_alias(
		"lib",
		target_command(
			output_file: out,
			dependencies: [raw],
			command: [
				'cargo',
				'run',
				raw.output_file,
				out
			],
			cwd: cur_src.join('patch-lib')
		)
	) {
		meta.output_file = out
	}
}
