use anyhow::Result;

mod implementation {
    use std::convert::Infallible;

    use anyhow::Result;
    use wasm_encoder::reencode::*;
    use wasm_encoder::*;

    const ROUND_MIN: i32 = 2; // softfloat_round_min
    const ROUND_MAX: i32 = 3; // softfloat_round_max
    const ROUND_MIN_MAG: i32 = 1; // softfloat_round_minMag

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
            to_i32: u32,
            to_i64: u32,

            round: u32,
        }

        #[derive(Clone, Copy)]
        struct FOps {
            add: u32,
            mul: u32,
            sub: u32,
            div: u32,
            lt: u32,
            le: u32,
            eq: u32,
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

            round: u32,
            sqrt: u32,
        }

        let mut f32_types: Option<FTypes> = None;
        let mut f32_ops: Option<FOps> = None;

        let mut f32_to_f64_type: Option<u32> = None;
        let mut f64_to_f32_type: Option<u32> = None;

        let mut f64_types: Option<FTypes> = None;
        let mut f64_ops: Option<FOps> = None;

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
                        to_i32: adder
                            .add([ValType::F32, ValType::I32, ValType::I32], [ValType::I32]),
                        to_i64: adder
                            .add([ValType::F32, ValType::I32, ValType::I32], [ValType::I64]),

                        round: adder
                            .add([ValType::F32, ValType::I32, ValType::I32], [ValType::F32]),
                    });
                    f64_types = Some(FTypes {
                        bopf: adder.add([ValType::F64, ValType::F64], [ValType::F64]),
                        uopf: adder.add([ValType::F64], [ValType::F64]),
                        boolop: adder.add([ValType::F64, ValType::F64], [ValType::I32]),
                        from_i32: adder.add([ValType::I32], [ValType::F64]),
                        from_i64: adder.add([ValType::I64], [ValType::F64]),
                        to_i32: adder
                            .add([ValType::F64, ValType::I32, ValType::I32], [ValType::I32]),
                        to_i64: adder
                            .add([ValType::F64, ValType::I32, ValType::I32], [ValType::I64]),

                        round: adder
                            .add([ValType::F64, ValType::I32, ValType::I32], [ValType::F64]),
                    });

                    f32_to_f64_type = Some(adder.add([ValType::F32], [ValType::F64]));
                    f64_to_f32_type = Some(adder.add([ValType::F64], [ValType::F32]));

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
                            self.cnt += 1;
                            let ret = self.imports.len();
                            self.imports
                                .import("softfloat", name, EntityType::Function(fn_type));
                            ret
                        }
                    }
                    let mut adder = Adder { cnt: 0, imports };
                    f32_ops = Some(FOps {
                        add: adder.add("f32_add", f32_types.unwrap().bopf),
                        mul: adder.add("f32_mul", f32_types.unwrap().bopf),
                        sub: adder.add("f32_sub", f32_types.unwrap().bopf),
                        div: adder.add("f32_div", f32_types.unwrap().bopf),
                        lt: adder.add("f32_lt_quiet", f32_types.unwrap().boolop),
                        le: adder.add("f32_le_quiet", f32_types.unwrap().boolop),
                        eq: adder.add("f32_eq", f32_types.unwrap().boolop),
                        ge: adder.add("f32_ge_quiet", f32_types.unwrap().boolop),
                        gt: adder.add("f32_gt_quiet", f32_types.unwrap().boolop),
                        from_u32: adder.add("ui32_to_f32", f32_types.unwrap().from_i32),
                        from_i32: adder.add("i32_to_f32", f32_types.unwrap().from_i32),
                        from_u64: adder.add("ui64_to_f32", f32_types.unwrap().from_i64),
                        from_i64: adder.add("i64_to_f32", f32_types.unwrap().from_i64),
                        to_i32: adder.add("f32_to_i32", f32_types.unwrap().to_i32),
                        to_u32: adder.add("f32_to_ui32", f32_types.unwrap().to_i32),
                        to_i64: adder.add("f32_to_i64", f32_types.unwrap().to_i64),
                        to_u64: adder.add("f32_to_ui64", f32_types.unwrap().to_i64),

                        flt_conv_to: adder.add("f64_to_f32", f64_to_f32_type.unwrap()),

                        round: adder.add("f32_roundToInt", f32_types.unwrap().round),
                        sqrt: adder.add("f32_sqrt", f32_types.unwrap().uopf),
                    });
                    f64_ops = Some(FOps {
                        add: adder.add("f64_add", f64_types.unwrap().bopf),
                        mul: adder.add("f64_mul", f64_types.unwrap().bopf),
                        sub: adder.add("f64_sub", f64_types.unwrap().bopf),
                        div: adder.add("f64_div", f64_types.unwrap().bopf),
                        lt: adder.add("f64_lt_quiet", f64_types.unwrap().boolop),
                        le: adder.add("f64_le_quiet", f64_types.unwrap().boolop),
                        eq: adder.add("f64_eq", f64_types.unwrap().boolop),
                        ge: adder.add("f64_ge_quiet", f64_types.unwrap().boolop),
                        gt: adder.add("f64_gt_quiet", f64_types.unwrap().boolop),
                        from_u32: adder.add("ui32_to_f64", f64_types.unwrap().from_i32),
                        from_i32: adder.add("i32_to_f64", f64_types.unwrap().from_i32),
                        from_u64: adder.add("ui64_to_f64", f64_types.unwrap().from_i64),
                        from_i64: adder.add("i64_to_f64", f64_types.unwrap().from_i64),
                        to_i32: adder.add("f64_to_i32", f64_types.unwrap().to_i32),
                        to_u32: adder.add("f64_to_ui32", f64_types.unwrap().to_i32),
                        to_i64: adder.add("f64_to_i64", f64_types.unwrap().to_i64),
                        to_u64: adder.add("f64_to_ui64", f64_types.unwrap().to_i64),

                        flt_conv_to: adder.add("f32_to_f64", f32_to_f64_type.unwrap()),

                        round: adder.add("f64_roundToInt", f64_types.unwrap().round),
                        sqrt: adder.add("f64_sqrt", f64_types.unwrap().uopf),
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
                                        f.instruction(&Instruction::I32ReinterpretF32);
                                        f.instruction(&Instruction::I32Const(i32::MIN));
                                        f.instruction(&Instruction::I32Xor);
                                        f.instruction(&Instruction::F32ReinterpretI32)
                                    }
                                    Instruction::F32Abs => {
                                        f.instruction(&Instruction::I32ReinterpretF32);
                                        f.instruction(&Instruction::I32Const(i32::MAX));
                                        f.instruction(&Instruction::I32And);
                                        f.instruction(&Instruction::F32ReinterpretI32)
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
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().eq));
                                        f.instruction(&Instruction::I32Const(1));
                                        f.instruction(&Instruction::I32Xor)
                                    }
                                    Instruction::I32TruncF32S => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_i32))
                                    }
                                    Instruction::I32TruncF32U => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_u32))
                                    }
                                    Instruction::I64TruncF32S => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_i64))
                                    }
                                    Instruction::I64TruncF32U => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().to_u64))
                                    }

                                    Instruction::F32Floor => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().round))
                                    }
                                    Instruction::F32Ceil => {
                                        f.instruction(&Instruction::I32Const(ROUND_MAX));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f32_ops.unwrap().round))
                                    }

                                    // f64
                                    Instruction::F64Neg => {
                                        f.instruction(&Instruction::I64ReinterpretF64);
                                        f.instruction(&Instruction::I64Const(i64::MIN));
                                        f.instruction(&Instruction::I64Xor);
                                        f.instruction(&Instruction::F64ReinterpretI64)
                                    }
                                    Instruction::F64Abs => {
                                        f.instruction(&Instruction::I64ReinterpretF64);
                                        f.instruction(&Instruction::I64Const(i64::MAX));
                                        f.instruction(&Instruction::I64And);
                                        f.instruction(&Instruction::F64ReinterpretI64)
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
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().eq));
                                        f.instruction(&Instruction::I32Const(1));
                                        f.instruction(&Instruction::I32Xor)
                                    }
                                    Instruction::I64TruncF64S => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_i64))
                                    }
                                    Instruction::I64TruncF64U => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_u64))
                                    }
                                    Instruction::I32TruncF64S => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_i32))
                                    }
                                    Instruction::I32TruncF64U => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN_MAG));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().to_u32))
                                    }

                                    Instruction::F64Floor => {
                                        f.instruction(&Instruction::I32Const(ROUND_MIN));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().round))
                                    }
                                    Instruction::F64Ceil => {
                                        f.instruction(&Instruction::I32Const(ROUND_MAX));
                                        f.instruction(&Instruction::I32Const(0));
                                        f.instruction(&Instruction::Call(f64_ops.unwrap().round))
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
