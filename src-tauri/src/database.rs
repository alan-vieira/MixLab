use rusqlite::{Connection, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub struct AnalysisDatabase {
    conn: Connection,
}

#[derive(Debug, Clone)] // ✅ ADICIONADO: Clone é obrigatório para usar em threads
pub struct CachedAnalysis {
    pub bpm: f32,
    pub duration_secs: f64,
    pub waveform_peaks: Vec<f32>,
    pub waveform_full_min: Vec<f32>, // ✅ NOVO
    pub waveform_full_max: Vec<f32>, // ✅ NOVO
    pub waveform_low: Vec<f32>,
    pub waveform_mid: Vec<f32>,
    pub waveform_high: Vec<f32>,
}

impl AnalysisDatabase {
    pub fn new() -> Result<Self> {
        // ✅ LÓGICA DE CAMINHO À PROVA DE FALHAS:
        // Garante que o DB fique na raiz do projeto, nunca dentro de src-tauri
        let mut db_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if db_path.ends_with("src-tauri") {
            db_path.pop(); // Sobe um nível se o terminal estiver dentro de src-tauri
        }
        db_path.push("mixlab_analysis.db");

        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS track_analysis (
                file_hash TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                bpm REAL NOT NULL,
                duration_secs REAL NOT NULL,
                waveform_peaks BLOB NOT NULL,
                waveform_full_min BLOB NOT NULL,
                waveform_full_max BLOB NOT NULL,
                waveform_low BLOB NOT NULL,
                waveform_mid BLOB NOT NULL,
                waveform_high BLOB NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(AnalysisDatabase { conn })
    }

    fn calculate_file_hash(path: &str) -> Result<String, std::io::Error> {
        let metadata = fs::metadata(path)?;
        let len = metadata.len();
        let mut hasher = Sha256::new();
        hasher.update(len.to_le_bytes());
        let mut file = fs::File::open(path)?;
        let mut buffer = [0u8; 65536];
        if let Ok(n) = file.read(&mut buffer) {
            hasher.update(&buffer[..n]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn get_cached(&self, path: &str) -> Result<Option<CachedAnalysis>> {
        let file_hash = match Self::calculate_file_hash(path) {
            Ok(hash) => hash,
            Err(_) => return Ok(None),
        };

        match self.conn.query_row(
            "SELECT bpm, duration_secs, waveform_peaks, waveform_full_min, waveform_full_max, waveform_low, waveform_mid, waveform_high 
             FROM track_analysis WHERE file_hash = ?1",
            [&file_hash],
            |row| {
                Ok(CachedAnalysis {
                    bpm: row.get(0)?,
                    duration_secs: row.get(1)?,
                    waveform_peaks: blob_to_f32_vec(&row.get::<_, Vec<u8>>(2)?),
                    waveform_full_min: blob_to_f32_vec(&row.get::<_, Vec<u8>>(3)?),
                    waveform_full_max: blob_to_f32_vec(&row.get::<_, Vec<u8>>(4)?),
                    waveform_low: blob_to_f32_vec(&row.get::<_, Vec<u8>>(5)?),
                    waveform_mid: blob_to_f32_vec(&row.get::<_, Vec<u8>>(6)?),
                    waveform_high: blob_to_f32_vec(&row.get::<_, Vec<u8>>(7)?),
                })
            },
        ) {
            Ok(analysis) => Ok(Some(analysis)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn save_analysis(&self, path: &str, analysis: &CachedAnalysis) -> Result<()> {
        let file_hash = match Self::calculate_file_hash(path) {
            Ok(hash) => hash,
            Err(_) => return Ok(()),
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO track_analysis 
             (file_hash, file_path, bpm, duration_secs, waveform_peaks, waveform_full_min, waveform_full_max, waveform_low, waveform_mid, waveform_high)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                &file_hash, path, &analysis.bpm, &analysis.duration_secs,
                &f32_vec_to_blob(&analysis.waveform_peaks),
                &f32_vec_to_blob(&analysis.waveform_full_min),
                &f32_vec_to_blob(&analysis.waveform_full_max),
                &f32_vec_to_blob(&analysis.waveform_low),
                &f32_vec_to_blob(&analysis.waveform_mid),
                &f32_vec_to_blob(&analysis.waveform_high),
            ),
        )?;
        Ok(())
    }
}

fn f32_vec_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vec.len() * 4);
    for &val in vec {
        blob.extend_from_slice(&val.to_ne_bytes());
    }
    blob
}

fn blob_to_f32_vec(blob: &[u8]) -> Vec<f32> {
    let mut vec = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks(4) {
        if chunk.len() == 4 {
            vec.push(f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
    }
    vec
}
