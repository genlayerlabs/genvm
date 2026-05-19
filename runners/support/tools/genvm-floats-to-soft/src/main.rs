use anyhow::Result;

mod implementation {
    use std::convert::Infallible;

    use anyhow::Result;
    use wasm_encoder::reencode::*;
    use wasm_encoder::*;

    pub struct MyEncoder {
        start: u32,
        off: u32,
    }

    impl MyEncoder {
        pub fn new() -> Self {
            Self { start: 0, off: 0 }
        }
    }

    impl Reencode for MyEncoder {
        type Error = Infallible;

        fn function_index(&mut self, func: u32) -> u32 {
            if func >= self.start {
                func + self.off
            } else {
                func
            }
        }
    }

    pub fn parse_core_module(
        reencoder: &mut MyEncoder,
        module: &mut Module,
        parser: wasmparser::Parser,
        data: &[u8],
    ) -> Result<(), Error<Infallible>> {
        fn handle_intersperse_section_hook<T: ?Sized + Reencode>(
            reencoder: &mut T,
            module: &mut Module,
            last_section: &mut Option<SectionId>,
            next_section: Option<SectionId>,
        ) -> Result<(), Error<T::Error>> {
            let after = std::mem::replace(last_section, next_section);
            let before = next_section;
            reencoder.intersperse_section_hook(module, after, before)
        }

        let mut sections = parser.parse_all(data);
        let mut next_section = sections.next();
        let mut last_section = None;

        #[derive(Clone, Copy)]
        struct FTypes {
            bopf: u32,
            uopf: u32,
            boolop: u32,
            from_i32: u32,
            from_i64: u32,
            sat_to_i32: u32,
            sat_to_i64: u32,
        }

        #[derive(Clone, Copy)]
        struct FOps {
            add: u32,
            mul: u32,
            sub: u32,
            div: u32,
            neg: u32,
            abs: u32,
            lt: u32,
            le: u32,
            eq: u32,
            ne: u32,
            ge: u32,
            gt: u32,
            from_u32: u32,
            from_i32: u32,
            from_u64: u32,
            from_i64: u32,
            to_u32: u32,
            to_i32: u32,
            to_u64: u32,
            to_i64: u32,

            flt_conv_to: u32,

            floor: u32,
            ceil: u32,
            trunc_float: u32,
            nearest: u32,
            sqrt: u32,

            copysign: u32,
            min: u32,
            max: u32,
            sat_to_i32: u32,
            sat_to_u32: u32,
            sat_to_i64: u32,
            sat_to_u64: u32,
        }

        #[derive(Clone, Copy)]
        struct SimdOps {
            f32x4_add: u32,
            f32x4_sub: u32,
            f32x4_mul: u32,
            f32x4_div: u32,
            f32x4_sqrt: u32,
            f32x4_neg: u32,
            f32x4_abs: u32,
            f32x4_min: u32,
            f32x4_max: u32,
            f32x4_pmin: u32,
            f32x4_pmax: u32,
            f32x4_ceil: u32,
            f32x4_floor: u32,
            f32x4_trunc: u32,
            f32x4_nearest: u32,
            f32x4_eq: u32,
            f32x4_ne: u32,
            f32x4_lt: u32,
            f32x4_gt: u32,
            f32x4_le: u32,
            f32x4_ge: u32,
            f32x4_convert_i32x4_s: u32,
            f32x4_convert_i32x4_u: u32,
            f32x4_demote_f64x2_zero: u32,

            f64x2_add: u32,
            f64x2_sub: u32,
            f64x2_mul: u32,
            f64x2_div: u32,
            f64x2_sqrt: u32,
            f64x2_neg: u32,
            f64x2_abs: u32,
            f64x2_min: u32,
            f64x2_max: u32,
            f64x2_pmin: u32,
            f64x2_pmax: u32,
            f64x2_ceil: u32,
            f64x2_floor: u32,
            f64x2_trunc: u32,
            f64x2_nearest: u32,
            f64x2_eq: u32,
            f64x2_ne: u32,
            f64x2_lt: u32,
            f64x2_gt: u32,
            f64x2_le: u32,
            f64x2_ge: u32,
            f64x2_convert_low_i32x4_s: u32,
            f64x2_convert_low_i32x4_u: u32,
            f64x2_promote_low_f32x4: u32,

            i32x4_trunc_sat_f32x4_s: u32,
            i32x4_trunc_sat_f32x4_u: u32,
            i32x4_trunc_sat_f64x2_s_zero: u32,
            i32x4_trunc_sat_f64x2_u_zero: u32,
        }

        let mut f32_types: Option<FTypes> = None;
        let mut f32_ops: Option<FOps> = None;

        let mut f32_to_f64_type: Option<u32> = None;
        let mut f64_to_f32_type: Option<u32> = None;

        let mut f64_types: Option<FTypes> = None;
        let mut f64_ops: Option<FOps> = None;

        let mut simd_bop_type: Option<u32> = None;
        let mut simd_uop_type: Option<u32> = None;
        let mut simd_ops: Option<SimdOps> = None;

        'outer: while let Some(section) = next_section {
            match section? {
                wasmparser::Payload::Version {
                    encoding: wasmparser::Encoding::Module,
                    ..
                } => {}
                wasmparser::Payload::Version { .. } => {
                    return Err(Error::UnexpectedNonCoreModuleSection)
                }
                wasmparser::Payload::TypeSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Type),
                    )?;
                    let mut types = TypeSection::new();
                    reencoder.parse_type_section(&mut types, section)?;

                    struct Adder {
                        types: TypeSection,
                    }
                    impl Adder {
                        fn add<P, R>(&mut self, par: P, res: R) -> u32
                        where
                            P: IntoIterator<Item = ValType>,
                            P::IntoIter: ExactSizeIterator,
                            R: IntoIterator<Item = ValType>,
                            R::IntoIter: ExactSizeIterator,
                        {
                            let ret = self.types.len();
                            self.types.function(par, res);
                            ret
                        }
                    }

                    let mut adder = Adder { types };
                    f32_types = Some(FTypes {
                        bopf: adder.add([ValType::F32, ValType::F32], [ValType::F32]),
                        uopf: adder.add([ValType::F32], [ValType::F32]),
                        boolop: adder.add([ValType::F32, ValType::F32], [ValType::I32]),
                        from_i32: adder.add([ValType::I32], [ValType::F32]),
                        from_i64: adder.add([ValType::I64], [ValType::F32]),
                        sat_to_i32: adder.add([ValType::F32], [ValType::I32]),
                        sat_to_i64: adder.add([ValType::F32], [ValType::I64]),
                    });
                    f64_types = Some(FTypes {
                        bopf: adder.add([ValType::F64, ValType::F64], [ValType::F64]),
                        uopf: adder.add([ValType::F64], [ValType::F64]),
                        boolop: adder.add([ValType::F64, ValType::F64], [ValType::I32]),
                        from_i32: adder.add([ValType::I32], [ValType::F64]),
                        from_i64: adder.add([ValType::I64], [ValType::F64]),
                        sat_to_i32: adder.add([ValType::F64], [ValType::I32]),
                        sat_to_i64: adder.add([ValType::F64], [ValType::I64]),
                    });

                    f32_to_f64_type = Some(adder.add([ValType::F32], [ValType::F64]));
                    f64_to_f32_type = Some(adder.add([ValType::F64], [ValType::F32]));

                    simd_bop_type =
                        Some(adder.add([ValType::V128, ValType::V128], [ValType::V128]));
                    simd_uop_type = Some(adder.add([ValType::V128], [ValType::V128]));

                    module.section(&adder.types);
                }
                wasmparser::Payload::ImportSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Import),
                    )?;
                    let mut imports = ImportSection::new();
                    reencoder.parse_import_section(&mut imports, section)?;

                    reencoder.start = imports.len();

                    struct Adder {
                        imports: ImportSection,
                        cnt: u32,
                    }
                    impl Adder {
                        fn add(&mut self, name: &str, fn_type: u32) -> u32 {
                            self.add_from("softfloat", name, fn_type)
                        }
                        fn add_from(&mut self, module: &str, name: &str, fn_type: u32) -> u32 {
                            self.cnt += 1;
                            let ret = self.imports.len();
                            self.imports
                                .import(module, name, EntityType::Function(fn_type));
                            ret
                        }
                    }
                    let mut adder = Adder { cnt: 0, imports };
                    f32_ops = Some(FOps {
                        add: adder.add("f32_add", f32_types.unwrap().bopf),
                        mul: adder.add("f32_mul", f32_types.unwrap().bopf),
                        sub: adder.add("f32_sub", f32_types.unwrap().bopf),
                        div: adder.add("f32_div", f32_types.unwrap().bopf),
                        neg: adder.add("f32_neg", f32_types.unwrap().uopf),
                        abs: adder.add("f32_abs", f32_types.unwrap().uopf),
                        lt: adder.add("f32_lt_quiet", f32_types.unwrap().boolop),
                        le: adder.add("f32_le_quiet", f32_types.unwrap().boolop),
                        eq: adder.add("f32_eq", f32_types.unwrap().boolop),
                        ne: adder.add("f32_ne", f32_types.unwrap().boolop),
                        ge: adder.add("f32_ge_quiet", f32_types.unwrap().boolop),
                        gt: adder.add("f32_gt_quiet", f32_types.unwrap().boolop),
                        from_u32: adder.add("ui32_to_f32", f32_types.unwrap().from_i32),
                        from_i32: adder.add("i32_to_f32", f32_types.unwrap().from_i32),
                        from_u64: adder.add("ui64_to_f32", f32_types.unwrap().from_i64),
                        from_i64: adder.add("i64_to_f32", f32_types.unwrap().from_i64),
                        to_i32: adder.add("f32_to_i32_trunc", f32_types.unwrap().sat_to_i32),
                        to_u32: adder.add("f32_to_ui32_trunc", f32_types.unwrap().sat_to_i32),
                        to_i64: adder.add("f32_to_i64_trunc", f32_types.unwrap().sat_to_i64),
                        to_u64: adder.add("f32_to_ui64_trunc", f32_types.unwrap().sat_to_i64),

                        flt_conv_to: adder.add("f64_to_f32", f64_to_f32_type.unwrap()),

                        floor: adder.add("f32_floor", f32_types.unwrap().uopf),
                        ceil: adder.add("f32_ceil", f32_types.unwrap().uopf),
                        trunc_float: adder.add("f32_trunc", f32_types.unwrap().uopf),
                        nearest: adder.add("f32_nearest", f32_types.unwrap().uopf),
                        sqrt: adder.add("f32_sqrt", f32_types.unwrap().uopf),

                        copysign: adder.add("f32_copysign", f32_types.unwrap().bopf),
                        min: adder.add("f32_min", f32_types.unwrap().bopf),
                        max: adder.add("f32_max", f32_types.unwrap().bopf),
                        sat_to_i32: adder.add("f32_to_i32_sat", f32_types.unwrap().sat_to_i32),
                        sat_to_u32: adder.add("f32_to_ui32_sat", f32_types.unwrap().sat_to_i32),
                        sat_to_i64: adder.add("f32_to_i64_sat", f32_types.unwrap().sat_to_i64),
                        sat_to_u64: adder.add("f32_to_ui64_sat", f32_types.unwrap().sat_to_i64),
                    });
                    f64_ops = Some(FOps {
                        add: adder.add("f64_add", f64_types.unwrap().bopf),
                        mul: adder.add("f64_mul", f64_types.unwrap().bopf),
                        sub: adder.add("f64_sub", f64_types.unwrap().bopf),
                        div: adder.add("f64_div", f64_types.unwrap().bopf),
                        neg: adder.add("f64_neg", f64_types.unwrap().uopf),
                        abs: adder.add("f64_abs", f64_types.unwrap().uopf),
                        lt: adder.add("f64_lt_quiet", f64_types.unwrap().boolop),
                        le: adder.add("f64_le_quiet", f64_types.unwrap().boolop),
                        eq: adder.add("f64_eq", f64_types.unwrap().boolop),
                        ne: adder.add("f64_ne", f64_types.unwrap().boolop),
                        ge: adder.add("f64_ge_quiet", f64_types.unwrap().boolop),
                        gt: adder.add("f64_gt_quiet", f64_types.unwrap().boolop),
                        from_u32: adder.add("ui32_to_f64", f64_types.unwrap().from_i32),
                        from_i32: adder.add("i32_to_f64", f64_types.unwrap().from_i32),
                        from_u64: adder.add("ui64_to_f64", f64_types.unwrap().from_i64),
                        from_i64: adder.add("i64_to_f64", f64_types.unwrap().from_i64),
                        to_i32: adder.add("f64_to_i32_trunc", f64_types.unwrap().sat_to_i32),
                        to_u32: adder.add("f64_to_ui32_trunc", f64_types.unwrap().sat_to_i32),
                        to_i64: adder.add("f64_to_i64_trunc", f64_types.unwrap().sat_to_i64),
                        to_u64: adder.add("f64_to_ui64_trunc", f64_types.unwrap().sat_to_i64),

                        flt_conv_to: adder.add("f32_to_f64", f32_to_f64_type.unwrap()),

                        floor: adder.add("f64_floor", f64_types.unwrap().uopf),
                        ceil: adder.add("f64_ceil", f64_types.unwrap().uopf),
                        trunc_float: adder.add("f64_trunc", f64_types.unwrap().uopf),
                        nearest: adder.add("f64_nearest", f64_types.unwrap().uopf),
                        sqrt: adder.add("f64_sqrt", f64_types.unwrap().uopf),

                        copysign: adder.add("f64_copysign", f64_types.unwrap().bopf),
                        min: adder.add("f64_min", f64_types.unwrap().bopf),
                        max: adder.add("f64_max", f64_types.unwrap().bopf),
                        sat_to_i32: adder.add("f64_to_i32_sat", f64_types.unwrap().sat_to_i32),
                        sat_to_u32: adder.add("f64_to_ui32_sat", f64_types.unwrap().sat_to_i32),
                        sat_to_i64: adder.add("f64_to_i64_sat", f64_types.unwrap().sat_to_i64),
                        sat_to_u64: adder.add("f64_to_ui64_sat", f64_types.unwrap().sat_to_i64),
                    });

                    // SIMD imports
                    let simd_bop = simd_bop_type.unwrap();
                    let simd_uop = simd_uop_type.unwrap();
                    let sm = "softfloat";
                    simd_ops = Some(SimdOps {
                        f32x4_add: adder.add_from(sm, "f32x4_add", simd_bop),
                        f32x4_sub: adder.add_from(sm, "f32x4_sub", simd_bop),
                        f32x4_mul: adder.add_from(sm, "f32x4_mul", simd_bop),
                        f32x4_div: adder.add_from(sm, "f32x4_div", simd_bop),
                        f32x4_sqrt: adder.add_from(sm, "f32x4_sqrt", simd_uop),
                        f32x4_neg: adder.add_from(sm, "f32x4_neg", simd_uop),
                        f32x4_abs: adder.add_from(sm, "f32x4_abs", simd_uop),
                        f32x4_min: adder.add_from(sm, "f32x4_min", simd_bop),
                        f32x4_max: adder.add_from(sm, "f32x4_max", simd_bop),
                        f32x4_pmin: adder.add_from(sm, "f32x4_pmin", simd_bop),
                        f32x4_pmax: adder.add_from(sm, "f32x4_pmax", simd_bop),
                        f32x4_ceil: adder.add_from(sm, "f32x4_ceil", simd_uop),
                        f32x4_floor: adder.add_from(sm, "f32x4_floor", simd_uop),
                        f32x4_trunc: adder.add_from(sm, "f32x4_trunc", simd_uop),
                        f32x4_nearest: adder.add_from(sm, "f32x4_nearest", simd_uop),
                        f32x4_eq: adder.add_from(sm, "f32x4_eq", simd_bop),
                        f32x4_ne: adder.add_from(sm, "f32x4_ne", simd_bop),
                        f32x4_lt: adder.add_from(sm, "f32x4_lt", simd_bop),
                        f32x4_gt: adder.add_from(sm, "f32x4_gt", simd_bop),
                        f32x4_le: adder.add_from(sm, "f32x4_le", simd_bop),
                        f32x4_ge: adder.add_from(sm, "f32x4_ge", simd_bop),
                        f32x4_convert_i32x4_s: adder.add_from(
                            sm,
                            "f32x4_convert_i32x4_s",
                            simd_uop,
                        ),
                        f32x4_convert_i32x4_u: adder.add_from(
                            sm,
                            "f32x4_convert_i32x4_u",
                            simd_uop,
                        ),
                        f32x4_demote_f64x2_zero: adder.add_from(
                            sm,
                            "f32x4_demote_f64x2_zero",
                            simd_uop,
                        ),

                        f64x2_add: adder.add_from(sm, "f64x2_add", simd_bop),
                        f64x2_sub: adder.add_from(sm, "f64x2_sub", simd_bop),
                        f64x2_mul: adder.add_from(sm, "f64x2_mul", simd_bop),
                        f64x2_div: adder.add_from(sm, "f64x2_div", simd_bop),
                        f64x2_sqrt: adder.add_from(sm, "f64x2_sqrt", simd_uop),
                        f64x2_neg: adder.add_from(sm, "f64x2_neg", simd_uop),
                        f64x2_abs: adder.add_from(sm, "f64x2_abs", simd_uop),
                        f64x2_min: adder.add_from(sm, "f64x2_min", simd_bop),
                        f64x2_max: adder.add_from(sm, "f64x2_max", simd_bop),
                        f64x2_pmin: adder.add_from(sm, "f64x2_pmin", simd_bop),
                        f64x2_pmax: adder.add_from(sm, "f64x2_pmax", simd_bop),
                        f64x2_ceil: adder.add_from(sm, "f64x2_ceil", simd_uop),
                        f64x2_floor: adder.add_from(sm, "f64x2_floor", simd_uop),
                        f64x2_trunc: adder.add_from(sm, "f64x2_trunc", simd_uop),
                        f64x2_nearest: adder.add_from(sm, "f64x2_nearest", simd_uop),
                        f64x2_eq: adder.add_from(sm, "f64x2_eq", simd_bop),
                        f64x2_ne: adder.add_from(sm, "f64x2_ne", simd_bop),
                        f64x2_lt: adder.add_from(sm, "f64x2_lt", simd_bop),
                        f64x2_gt: adder.add_from(sm, "f64x2_gt", simd_bop),
                        f64x2_le: adder.add_from(sm, "f64x2_le", simd_bop),
                        f64x2_ge: adder.add_from(sm, "f64x2_ge", simd_bop),
                        f64x2_convert_low_i32x4_s: adder.add_from(
                            sm,
                            "f64x2_convert_low_i32x4_s",
                            simd_uop,
                        ),
                        f64x2_convert_low_i32x4_u: adder.add_from(
                            sm,
                            "f64x2_convert_low_i32x4_u",
                            simd_uop,
                        ),
                        f64x2_promote_low_f32x4: adder.add_from(
                            sm,
                            "f64x2_promote_low_f32x4",
                            simd_uop,
                        ),

                        i32x4_trunc_sat_f32x4_s: adder.add_from(
                            sm,
                            "i32x4_trunc_sat_f32x4_s",
                            simd_uop,
                        ),
                        i32x4_trunc_sat_f32x4_u: adder.add_from(
                            sm,
                            "i32x4_trunc_sat_f32x4_u",
                            simd_uop,
                        ),
                        i32x4_trunc_sat_f64x2_s_zero: adder.add_from(
                            sm,
                            "i32x4_trunc_sat_f64x2_s_zero",
                            simd_uop,
                        ),
                        i32x4_trunc_sat_f64x2_u_zero: adder.add_from(
                            sm,
                            "i32x4_trunc_sat_f64x2_u_zero",
                            simd_uop,
                        ),
                    });
                    reencoder.off = adder.cnt;

                    module.section(&adder.imports);
                }
                wasmparser::Payload::FunctionSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Function),
                    )?;
                    let mut functions = FunctionSection::new();
                    reencoder.parse_function_section(&mut functions, section)?;
                    module.section(&functions);
                }
                wasmparser::Payload::TableSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Table),
                    )?;
                    let mut tables = TableSection::new();
                    reencoder.parse_table_section(&mut tables, section)?;
                    module.section(&tables);
                }
                wasmparser::Payload::MemorySection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Memory),
                    )?;
                    let mut memories = MemorySection::new();
                    reencoder.parse_memory_section(&mut memories, section)?;
                    module.section(&memories);
                }
                wasmparser::Payload::TagSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Tag),
                    )?;
                    let mut tags = TagSection::new();
                    reencoder.parse_tag_section(&mut tags, section)?;
                    module.section(&tags);
                }
                wasmparser::Payload::GlobalSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Global),
                    )?;
                    let mut globals = GlobalSection::new();
                    reencoder.parse_global_section(&mut globals, section)?;
                    module.section(&globals);
                }
                wasmparser::Payload::ExportSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Export),
                    )?;
                    let mut exports = ExportSection::new();
                    reencoder.parse_export_section(&mut exports, section)?;
                    module.section(&exports);
                }
                wasmparser::Payload::StartSection { func, .. } => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Start),
                    )?;
                    module.section(&StartSection {
                        function_index: reencoder.function_index(func),
                    });
                }
                wasmparser::Payload::ElementSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Element),
                    )?;
                    let mut elements = ElementSection::new();
                    reencoder.parse_element_section(&mut elements, section)?;
                    module.section(&elements);
                }
                wasmparser::Payload::DataCountSection { count, .. } => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::DataCount),
                    )?;
                    module.section(&DataCountSection { count });
                }
                wasmparser::Payload::DataSection(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Data),
                    )?;
                    let mut data = DataSection::new();
                    reencoder.parse_data_section(&mut data, section)?;
                    module.section(&data);
                }
                wasmparser::Payload::CodeSectionStart { count, .. } => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Code),
                    )?;
                    let mut codes = CodeSection::new();
                    for _ in 0..count {
                        if let Some(Ok(wasmparser::Payload::CodeSectionEntry(section))) =
                            sections.next()
                        {
                            let mut f = reencoder.new_function_with_parsed_locals(&section)?;
                            let mut reader = section.get_operators_reader()?;
                            while !reader.eof() {
                                let ins = reencoder.parse_instruction(&mut reader)?;
                                match ins {
                                    // f32
                                    Instruction::F32Neg => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().neg))
                                    }
                                    Instruction::F32Abs => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().abs))
                                    }
                                    Instruction::F32Copysign => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().copysign))
                                    }
                                    Instruction::F32ConvertI32U => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().from_u32))
                                    }
                                    Instruction::F32ConvertI32S => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().from_i32))
                                    }
                                    Instruction::F32ConvertI64U => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().from_u64))
                                    }
                                    Instruction::F32ConvertI64S => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().from_i64))
                                    }
                                    Instruction::F32Add => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().add))
                                    }
                                    Instruction::F32Sub => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().sub))
                                    }
                                    Instruction::F32Mul => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().mul))
                                    }
                                    Instruction::F32Div => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().div))
                                    }
                                    Instruction::F32Min => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().min))
                                    }
                                    Instruction::F32Max => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().max))
                                    }
                                    Instruction::F32Le => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().le))
                                    }
                                    Instruction::F32Lt => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().lt))
                                    }
                                    Instruction::F32Ge => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().ge))
                                    }
                                    Instruction::F32Gt => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().gt))
                                    }
                                    Instruction::F32Eq => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().eq))
                                    }
                                    Instruction::F32Ne => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().ne))
                                    }
                                    Instruction::I32TruncF32S => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_i32))
                                    }
                                    Instruction::I32TruncF32U => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_u32))
                                    }
                                    Instruction::I64TruncF32S => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_i64))
                                    }
                                    Instruction::I64TruncF32U => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_u64))
                                    }
                                    Instruction::I32TruncSatF32S => f.instruction(
                                        &Instruction::Call(f32_ops.unwrap().sat_to_i32),
                                    ),
                                    Instruction::I32TruncSatF32U => f.instruction(
                                        &Instruction::Call(f32_ops.unwrap().sat_to_u32),
                                    ),
                                    Instruction::I64TruncSatF32S => f.instruction(
                                        &Instruction::Call(f32_ops.unwrap().sat_to_i64),
                                    ),
                                    Instruction::I64TruncSatF32U => f.instruction(
                                        &Instruction::Call(f32_ops.unwrap().sat_to_u64),
                                    ),

                                    Instruction::F32Floor => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().floor))
                                    }
                                    Instruction::F32Ceil => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().ceil))
                                    }
                                    Instruction::F32Trunc => f.instruction(&Instruction::Call(
                                        f32_ops.unwrap().trunc_float,
                                    )),
                                    Instruction::F32Nearest => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().nearest))
                                    }

                                    // f64
                                    Instruction::F64Neg => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().neg))
                                    }
                                    Instruction::F64Abs => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().abs))
                                    }
                                    Instruction::F64Copysign => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().copysign))
                                    }
                                    Instruction::F64ConvertI32U => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().from_u32))
                                    }
                                    Instruction::F64ConvertI32S => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().from_i32))
                                    }
                                    Instruction::F64ConvertI64U => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().from_u64))
                                    }
                                    Instruction::F64ConvertI64S => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().from_i64))
                                    }
                                    Instruction::F64Add => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().add))
                                    }
                                    Instruction::F64Sub => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().sub))
                                    }
                                    Instruction::F64Mul => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().mul))
                                    }
                                    Instruction::F64Div => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().div))
                                    }
                                    Instruction::F64Min => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().min))
                                    }
                                    Instruction::F64Max => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().max))
                                    }
                                    Instruction::F64Le => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().le))
                                    }
                                    Instruction::F64Lt => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().lt))
                                    }
                                    Instruction::F64Ge => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().ge))
                                    }
                                    Instruction::F64Gt => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().gt))
                                    }
                                    Instruction::F64Eq => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().eq))
                                    }
                                    Instruction::F64Ne => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().ne))
                                    }
                                    Instruction::I64TruncF64S => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_i64))
                                    }
                                    Instruction::I64TruncF64U => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_u64))
                                    }
                                    Instruction::I32TruncF64S => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_i32))
                                    }
                                    Instruction::I32TruncF64U => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_u32))
                                    }
                                    Instruction::I32TruncSatF64S => f.instruction(
                                        &Instruction::Call(f64_ops.unwrap().sat_to_i32),
                                    ),
                                    Instruction::I32TruncSatF64U => f.instruction(
                                        &Instruction::Call(f64_ops.unwrap().sat_to_u32),
                                    ),
                                    Instruction::I64TruncSatF64S => f.instruction(
                                        &Instruction::Call(f64_ops.unwrap().sat_to_i64),
                                    ),
                                    Instruction::I64TruncSatF64U => f.instruction(
                                        &Instruction::Call(f64_ops.unwrap().sat_to_u64),
                                    ),

                                    Instruction::F64Floor => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().floor))
                                    }
                                    Instruction::F64Ceil => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().ceil))
                                    }
                                    Instruction::F64Trunc => f.instruction(&Instruction::Call(
                                        f64_ops.unwrap().trunc_float,
                                    )),
                                    Instruction::F64Nearest => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().nearest))
                                    }
                                    Instruction::F32DemoteF64 => f.instruction(&Instruction::Call(
                                        f32_ops.unwrap().flt_conv_to,
                                    )),
                                    Instruction::F64PromoteF32 => f.instruction(
                                        &Instruction::Call(f64_ops.unwrap().flt_conv_to),
                                    ),

                                    Instruction::F32Sqrt => {
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().sqrt))
                                    }
                                    Instruction::F64Sqrt => {
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().sqrt))
                                    }

                                    // SIMD f32x4
                                    Instruction::F32x4Add => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_add,
                                    )),
                                    Instruction::F32x4Sub => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_sub,
                                    )),
                                    Instruction::F32x4Mul => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_mul,
                                    )),
                                    Instruction::F32x4Div => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_div,
                                    )),
                                    Instruction::F32x4Sqrt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_sqrt,
                                    )),
                                    Instruction::F32x4Neg => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_neg,
                                    )),
                                    Instruction::F32x4Abs => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_abs,
                                    )),
                                    Instruction::F32x4Min => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_min,
                                    )),
                                    Instruction::F32x4Max => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_max,
                                    )),
                                    Instruction::F32x4PMin => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_pmin,
                                    )),
                                    Instruction::F32x4PMax => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_pmax,
                                    )),
                                    Instruction::F32x4Ceil => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_ceil,
                                    )),
                                    Instruction::F32x4Floor => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_floor,
                                    )),
                                    Instruction::F32x4Trunc => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_trunc,
                                    )),
                                    Instruction::F32x4Nearest => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_nearest,
                                    )),
                                    Instruction::F32x4Eq => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_eq,
                                    )),
                                    Instruction::F32x4Ne => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_ne,
                                    )),
                                    Instruction::F32x4Lt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_lt,
                                    )),
                                    Instruction::F32x4Gt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_gt,
                                    )),
                                    Instruction::F32x4Le => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_le,
                                    )),
                                    Instruction::F32x4Ge => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f32x4_ge,
                                    )),
                                    Instruction::F32x4ConvertI32x4S => f.instruction(
                                        &Instruction::Call(simd_ops.unwrap().f32x4_convert_i32x4_s),
                                    ),
                                    Instruction::F32x4ConvertI32x4U => f.instruction(
                                        &Instruction::Call(simd_ops.unwrap().f32x4_convert_i32x4_u),
                                    ),
                                    Instruction::F32x4DemoteF64x2Zero => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().f32x4_demote_f64x2_zero,
                                        ))
                                    }

                                    // SIMD f64x2
                                    Instruction::F64x2Add => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_add,
                                    )),
                                    Instruction::F64x2Sub => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_sub,
                                    )),
                                    Instruction::F64x2Mul => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_mul,
                                    )),
                                    Instruction::F64x2Div => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_div,
                                    )),
                                    Instruction::F64x2Sqrt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_sqrt,
                                    )),
                                    Instruction::F64x2Neg => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_neg,
                                    )),
                                    Instruction::F64x2Abs => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_abs,
                                    )),
                                    Instruction::F64x2Min => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_min,
                                    )),
                                    Instruction::F64x2Max => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_max,
                                    )),
                                    Instruction::F64x2PMin => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_pmin,
                                    )),
                                    Instruction::F64x2PMax => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_pmax,
                                    )),
                                    Instruction::F64x2Ceil => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_ceil,
                                    )),
                                    Instruction::F64x2Floor => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_floor,
                                    )),
                                    Instruction::F64x2Trunc => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_trunc,
                                    )),
                                    Instruction::F64x2Nearest => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_nearest,
                                    )),
                                    Instruction::F64x2Eq => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_eq,
                                    )),
                                    Instruction::F64x2Ne => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_ne,
                                    )),
                                    Instruction::F64x2Lt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_lt,
                                    )),
                                    Instruction::F64x2Gt => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_gt,
                                    )),
                                    Instruction::F64x2Le => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_le,
                                    )),
                                    Instruction::F64x2Ge => f.instruction(&Instruction::Call(
                                        simd_ops.unwrap().f64x2_ge,
                                    )),
                                    Instruction::F64x2ConvertLowI32x4S => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().f64x2_convert_low_i32x4_s,
                                        ))
                                    }
                                    Instruction::F64x2ConvertLowI32x4U => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().f64x2_convert_low_i32x4_u,
                                        ))
                                    }
                                    Instruction::F64x2PromoteLowF32x4 => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().f64x2_promote_low_f32x4,
                                        ))
                                    }

                                    // SIMD truncation
                                    Instruction::I32x4TruncSatF32x4S => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().i32x4_trunc_sat_f32x4_s,
                                        ))
                                    }
                                    Instruction::I32x4TruncSatF32x4U => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().i32x4_trunc_sat_f32x4_u,
                                        ))
                                    }
                                    Instruction::I32x4TruncSatF64x2SZero => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().i32x4_trunc_sat_f64x2_s_zero,
                                        ))
                                    }
                                    Instruction::I32x4TruncSatF64x2UZero => {
                                        f.instruction(&Instruction::Call(
                                            simd_ops.unwrap().i32x4_trunc_sat_f64x2_u_zero,
                                        ))
                                    }

                                    // common
                                    ins => f.instruction(&ins),
                                };
                            }
                            codes.function(&f);
                        } else {
                            return Err(Error::UnexpectedNonCoreModuleSection);
                        }
                    }
                    module.section(&codes);
                }
                wasmparser::Payload::CodeSectionEntry(section) => {
                    handle_intersperse_section_hook(
                        reencoder,
                        module,
                        &mut last_section,
                        Some(SectionId::Code),
                    )?;
                    // we can't do better than start a new code section here
                    let mut codes = CodeSection::new();
                    reencoder.parse_function_body(&mut codes, section)?;

                    for section in sections.by_ref() {
                        let section = section?;
                        if let wasmparser::Payload::CodeSectionEntry(section) = section {
                            reencoder.parse_function_body(&mut codes, section)?;
                        } else {
                            module.section(&codes);
                            next_section = Some(Ok(section));
                            continue 'outer;
                        }
                    }
                    module.section(&codes);
                }
                wasmparser::Payload::ModuleSection { .. }
                | wasmparser::Payload::InstanceSection(_)
                | wasmparser::Payload::CoreTypeSection(_)
                | wasmparser::Payload::ComponentSection { .. }
                | wasmparser::Payload::ComponentInstanceSection(_)
                | wasmparser::Payload::ComponentAliasSection(_)
                | wasmparser::Payload::ComponentTypeSection(_)
                | wasmparser::Payload::ComponentCanonicalSection(_)
                | wasmparser::Payload::ComponentStartSection { .. }
                | wasmparser::Payload::ComponentImportSection(_)
                | wasmparser::Payload::ComponentExportSection(_) => {
                    return Err(Error::UnexpectedNonCoreModuleSection)
                }
                wasmparser::Payload::CustomSection(contents) => {
                    reencoder.parse_custom_section(module, contents)?;
                }
                wasmparser::Payload::UnknownSection { id, contents, .. } => {
                    reencoder.parse_unknown_section(module, id, contents)?;
                }
                wasmparser::Payload::End(_) => {
                    handle_intersperse_section_hook(reencoder, module, &mut last_section, None)?;
                }
            }

            next_section = sections.next();
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let in_file = args
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("no input provided"))?;
    let out_file = args
        .get(2)
        .ok_or_else(|| anyhow::anyhow!("no output provided"))?;

    let mut res_module = wasm_encoder::Module::new();
    let mut encoder = implementation::MyEncoder::new();

    let in_file = std::fs::read(in_file)?;
    let parser = wasmparser::Parser::new(0);
    implementation::parse_core_module(&mut encoder, &mut res_module, parser, &in_file)?;

    let bytes = res_module.finish();
    wasmparser::validate(&bytes)?;
    std::fs::write(out_file, &bytes)?;

    Ok(())
}
