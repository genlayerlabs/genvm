use anyhow::Result;

mod implementation {
use anyhow::Result;
use wasm_encoder::reencode::*;
use wasm_encoder::*;

fn patch_name_subs<T: ?Sized + Reencode>(
    reencoder: &mut T,
    module: &mut Module,
    section: wasmparser::Subsections<wasmparser::Name>,
    new_name: &str,
) -> Result<(), Error<T::Error>> {
    let mut ret = wasm_encoder::NameSection::new();
    ret.module(new_name);
    for subsection in section {
        match subsection? {
            wasmparser::Name::Module { .. } => {},
            subsection => reencoder.parse_custom_name_subsection(&mut ret, subsection)?,
        }
    }
    module.section(&ret);
    Ok(())
}

pub fn parse_core_module<T: ?Sized + Reencode>(
    reencoder: &mut T,
    module: &mut Module,
    parser: wasmparser::Parser,
    data: &[u8],
    new_name: &str,
) -> Result<(), Error<T::Error>> {
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

    'outer: while let Some(section) = next_section {
        match section? {
            wasmparser::Payload::Version {
                encoding: wasmparser::Encoding::Module,
                ..
            } => (),
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
                module.section(&types);
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
                module.section(&imports);
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
                for export in section {
                    let export = export?;
                    if !(export.name.contains("f64") || export.name.contains("f32")) {
                        continue;
                    }
                    exports.export(
                        export.name,
                        reencoder.export_kind(export.kind),
                        reencoder.external_index(export.kind, export.index),
                    );
                }
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
                        reencoder.parse_function_body(&mut codes, section)?;
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
            wasmparser::Payload::CustomSection(section) => {
                match section.as_known() {
                    wasmparser::KnownCustom::Name(name) => {
                        patch_name_subs(reencoder, module, name, new_name)?;
                    },
                    _ => reencoder.parse_custom_section(module, section)?,
                }
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
    let mod_name = "softfloat";

    let mut res_module = wasm_encoder::Module::new();
    let mut encoder = wasm_encoder::reencode::RoundtripReencoder{};

    let in_file = std::fs::read(in_file)?;
    let parser = wasmparser::Parser::new(0);
    implementation::parse_core_module(&mut encoder, &mut res_module, parser, &in_file, &mod_name)?;

    std::fs::write(out_file, res_module.finish())?;

    Ok(())
}
