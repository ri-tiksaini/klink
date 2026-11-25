use klink_common::{FileId, SymbolId};
use klink_index::{extract_symbol_refs, MatchKind, SymbolStore, SymbolTable, UnifiedSearch};
use klink_parse::{Span, SymbolDef, SymbolKind};

#[test]
fn gate_exact_prefix_fuzzy_xrefs_and_redb_roundtrip() {
    let defs = vec![
        SymbolDef {
            file: FileId(0),
            name: "tarjan_scc_iterative".into(),
            kind: SymbolKind::Function,
            span: Span { start: 0, end: 0 },
        },
        SymbolDef {
            file: FileId(1),
            name: "tarjan_helper".into(),
            kind: SymbolKind::Function,
            span: Span { start: 3, end: 3 },
        },
        SymbolDef {
            file: FileId(2),
            name: "brandes_betweenness".into(),
            kind: SymbolKind::Function,
            span: Span { start: 1, end: 1 },
        },
    ];

    let table = SymbolTable::from_defs(defs);
    let files = vec![
        (FileId(0), "fn tarjan_scc_iterative() {}\n".to_string()),
        (FileId(1), "fn tarjan_helper() { tarjan_scc_iterative(); }\n".to_string()),
        (FileId(2), "fn brandes_betweenness() { tarjan_scc_iterative(); }\n".to_string()),
    ];

    let xrefs = extract_symbol_refs(&table, &files);
    assert_eq!(xrefs.references(SymbolId(0)).len(), 2);

    let search = UnifiedSearch::build(&table);

    // Exact
    let exact = search.query(&table, "tarjan_helper", 5);
    assert_eq!(exact[0].match_kind, MatchKind::Exact);
    assert_eq!(exact[0].record.name, "tarjan_helper");

    // Prefix
    let pref = search.query(&table, "tarjan", 5);
    assert_eq!(pref.len(), 2);
    assert_eq!(pref[0].match_kind, MatchKind::Prefix);

    // Typo-tolerant fuzzy ("brandes_betweness" missing one 'e')
    let fuzzy = search.query(&table, "brandes_betweness", 5);
    assert!(!fuzzy.is_empty());
    assert_eq!(fuzzy[0].match_kind, MatchKind::Fuzzy);
    assert_eq!(fuzzy[0].record.name, "brandes_betweenness");

    // redb round-trip persistence
    let db_path = std::env::temp_dir().join(format!("klink_p2_redb_{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    {
        let store = SymbolStore::open(&db_path).unwrap();
        let fpaths = vec!["scc.rs".into(), "helper.rs".into(), "centrality.rs".into()];
        store.write_all(&fpaths, &table, &xrefs).unwrap();
    }
    {
        let store = SymbolStore::open(&db_path).unwrap();
        let rec = store.get_symbol(SymbolId(0)).unwrap().unwrap();
        assert_eq!(rec.name, "tarjan_scc_iterative");
        let refs = store.get_xrefs(SymbolId(0)).unwrap();
        assert_eq!(refs.len(), 2);
    }
    let _ = std::fs::remove_file(&db_path);
}
