use crate::fact::{Fact, Span, SymbolDef, SymbolKind};
use hashbrown::HashSet;
use klink_common::{Confidence, EdgeKind, FileId};
use object::{Object, ObjectSection};

#[derive(Clone, Debug, Default)]
pub struct BinaryInfo {
    pub format: &'static str,
    pub has_dwarf: bool,
    pub pdb_path: Option<String>,
    pub import_count: usize,
    pub export_count: usize,
    pub warning: Option<String>,
}

/// Pure-Rust PE / ELF / Mach-O import, export, and debug-marker extractor (`object` crate).
/// Never panics on malformed or truncated binaries; degrades gracefully with a warning.
pub fn extract_binary_facts(
    file_id: FileId,
    bytes: &[u8],
) -> (Vec<Fact>, Vec<SymbolDef>, BinaryInfo) {
    let Ok(obj) = object::File::parse(bytes) else {
        return (
            Vec::new(),
            Vec::new(),
            BinaryInfo {
                format: "Unknown",
                warning: Some("malformed or truncated binary header".to_string()),
                ..Default::default()
            },
        );
    };

    let format = match obj.format() {
        object::BinaryFormat::Elf => "ELF",
        object::BinaryFormat::Pe => "PE",
        object::BinaryFormat::MachO => "Mach-O",
        _ => "Binary",
    };

    let mut facts = Vec::new();
    let mut syms = Vec::new();
    let mut seen_libs: HashSet<String> = HashSet::new();

    if let Ok(imports) = obj.imports() {
        for imp in imports {
            let lib = String::from_utf8_lossy(imp.library()).trim().to_string();
            if !lib.is_empty() && seen_libs.insert(lib.clone()) {
                facts.push(Fact {
                    source: file_id,
                    kind: EdgeKind::BinaryImport,
                    raw_target: lib,
                    span: Span::default(),
                    confidence: Confidence::Static,
                });
            }
        }
    }

    let mut export_count = 0usize;
    if let Ok(exports) = obj.exports() {
        for exp in exports {
            let name = String::from_utf8_lossy(exp.name()).trim().to_string();
            if !name.is_empty() {
                export_count += 1;
                if syms.len() < 512 {
                    syms.push(SymbolDef {
                        file: file_id,
                        name,
                        kind: SymbolKind::Function,
                        span: Span::default(),
                    });
                }
            }
        }
    }

    let has_dwarf = obj.section_by_name(".debug_line").is_some()
        || obj.section_by_name(".debug_info").is_some();

    let mut pdb_path = None;
    if let Ok(Some(cv)) = obj.pdb_info() {
        let p = String::from_utf8_lossy(cv.path()).trim().to_string();
        if !p.is_empty() {
            facts.push(Fact {
                source: file_id,
                kind: EdgeKind::BinaryImport,
                raw_target: p.clone(),
                span: Span::default(),
                confidence: Confidence::Heuristic,
            });
            pdb_path = Some(p);
        }
    }

    let info = BinaryInfo {
        format,
        has_dwarf,
        pdb_path,
        import_count: seen_libs.len(),
        export_count,
        warning: None,
    };

    (facts, syms, info)
}
