use crate::symbols::{SymbolRecord, SymbolTable};
use crate::xref::{XrefIndex, XrefSite};
use klink_common::{KlinkError, Result, SymbolId};
use redb::{Database, TableDefinition};
use std::path::Path;

const SYMBOLS_TABLE: TableDefinition<u32, &[u8]> = TableDefinition::new("symbols");
const XREFS_TABLE: TableDefinition<u32, &[u8]> = TableDefinition::new("xrefs");
const FILES_TABLE: TableDefinition<u32, &str> = TableDefinition::new("files");
const META_TABLE: TableDefinition<&str, &str> = TableDefinition::new("meta");

fn db_err<E: std::fmt::Display>(e: E) -> KlinkError {
    KlinkError::Serialization(e.to_string())
}

/// Persistent ACID B-tree store backed by pure-Rust `redb` (`.klink/symbols.redb`).
pub struct SymbolStore {
    db: Database,
}

impl SymbolStore {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Database::create(path).map_err(db_err)?;
        Ok(Self { db })
    }

    pub fn write_all(
        &self,
        file_paths: &[String],
        symbols: &SymbolTable,
        xrefs: &XrefIndex,
    ) -> Result<()> {
        let write_txn = self.db.begin_write().map_err(db_err)?;
        {
            let mut f_tbl = write_txn.open_table(FILES_TABLE).map_err(db_err)?;
            for (idx, p) in file_paths.iter().enumerate() {
                f_tbl.insert(idx as u32, p.as_str()).map_err(db_err)?;
            }

            let mut s_tbl = write_txn.open_table(SYMBOLS_TABLE).map_err(db_err)?;
            let mut x_tbl = write_txn.open_table(XREFS_TABLE).map_err(db_err)?;

            for idx in 0..symbols.len() {
                let sid = SymbolId(idx as u32);
                if let Some(rec) = symbols.get(sid) {
                    let bytes = serde_json::to_vec(&rec).map_err(db_err)?;
                    s_tbl.insert(idx as u32, bytes.as_slice()).map_err(db_err)?;
                }
                let refs = xrefs.references(sid);
                if !refs.is_empty() {
                    let rbytes = serde_json::to_vec(refs).map_err(db_err)?;
                    x_tbl.insert(idx as u32, rbytes.as_slice()).map_err(db_err)?;
                }
            }

            let mut m_tbl = write_txn.open_table(META_TABLE).map_err(db_err)?;
            m_tbl.insert("schema_version", "1").map_err(db_err)?;
            let sym_cnt = symbols.len().to_string();
            m_tbl.insert("symbol_count", sym_cnt.as_str()).map_err(db_err)?;
        }
        write_txn.commit().map_err(db_err)?;
        Ok(())
    }

    pub fn get_symbol(&self, sid: SymbolId) -> Result<Option<SymbolRecord>> {
        let read_txn = self.db.begin_read().map_err(db_err)?;
        let tbl = read_txn.open_table(SYMBOLS_TABLE).map_err(db_err)?;
        match tbl.get(sid.as_u32()).map_err(db_err)? {
            Some(guard) => {
                let rec: SymbolRecord = serde_json::from_slice(guard.value()).map_err(db_err)?;
                Ok(Some(rec))
            }
            None => Ok(None),
        }
    }

    pub fn get_xrefs(&self, sid: SymbolId) -> Result<Vec<XrefSite>> {
        let read_txn = self.db.begin_read().map_err(db_err)?;
        let tbl = read_txn.open_table(XREFS_TABLE).map_err(db_err)?;
        match tbl.get(sid.as_u32()).map_err(db_err)? {
            Some(guard) => {
                let sites: Vec<XrefSite> = serde_json::from_slice(guard.value()).map_err(db_err)?;
                Ok(sites)
            }
            None => Ok(Vec::new()),
        }
    }
}
