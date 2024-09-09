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

        #[derive(Clone)]
        struct F64Types {
            bopf64: u32,
            uopf64: u32,
            boolopf64: u32,
            from_i32: u32,
            from_i64: u32,
        }
        impl Copy for F64Types {}
        #[derive(Clone)]
        struct F64Ops {
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
        }
        impl Copy for F64Ops {}
        let mut f64_types: Option<F64Types> = None;
        let mut f64_ops: Option<F64Ops> = None;

        //let mut

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
                    f64_types = Some(F64Types {
                        bopf64: adder.add([ValType::F64, ValType::F64], [ValType::F64]),
                        uopf64: adder.add([ValType::F64], [ValType::F64]),
                        boolopf64: adder.add([ValType::F64, ValType::F64], [ValType::I32]),
                        from_i32: adder.add([ValType::I32], [ValType::F64]),
                        from_i64: adder.add([ValType::I64], [ValType::F64]),
                    });

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
                    f64_ops = Some(F64Ops {
                        add: adder.add("f64_add", f64_types.unwrap().bopf64),
                        mul: adder.add("f64_mul", f64_types.unwrap().bopf64),
                        sub: adder.add("f64_sub", f64_types.unwrap().bopf64),
                        div: adder.add("f64_div", f64_types.unwrap().bopf64),
                        lt: adder.add("f64_lt_quiet", f64_types.unwrap().boolopf64),
                        le: adder.add("f64_le_quiet", f64_types.unwrap().boolopf64),
                        eq: adder.add("f64_eq", f64_types.unwrap().boolopf64),
                        ge: adder.add("f64_ge_quiet", f64_types.unwrap().boolopf64),
                        gt: adder.add("f64_gt_quiet", f64_types.unwrap().boolopf64),
                        from_u32: adder.add("ui32_to_f64", f64_types.unwrap().from_i32),
                        from_i32: adder.add("i32_to_f64", f64_types.unwrap().from_i32),
                        from_u64: adder.add("ui64_to_f64", f64_types.unwrap().from_i64),
                        from_i64: adder.add("i64_to_f64", f64_types.unwrap().from_i64),
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
                                    Instruction::F64Neg => {
                                        f.instruction(&Instruction::I64ReinterpretF64);
                                        f.instruction(&Instruction::I64Const(i64::min_value()));
                                        f.instruction(&Instruction::I64Xor);
                                        f.instruction(&Instruction::F64ReinterpretI64)
                                    }
                                    Instruction::F64Abs => {
                                        f.instruction(&Instruction::I64ReinterpretF64);
                                        f.instruction(&Instruction::I64Const(i64::max_value()));
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
                    while let Some(section) = sections.next() {
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
    let in_file = args.get(1).ok_or(anyhow::anyhow!("no input provided"))?;
    let out_file = args.get(2).ok_or(anyhow::anyhow!("no output provided"))?;

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
