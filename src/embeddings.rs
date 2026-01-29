//! Semantic Embeddings for 100minds
//!
//! Local embedding model using ONNX Runtime for vocabulary-mismatch-proof search.
//!
//! Architecture (Approach A+B from design):
//! 1. Pre-compute 384-dim embeddings for all principles (one-time)
//! 2. Embed query at runtime (~10ms)
//! 3. Cosine similarity for top-K candidates
//! 4. Combine with BM25 for hybrid ranking
//!
//! Model: all-MiniLM-L6-v2 (22MB, 384 dimensions, runs on CPU)

use anyhow::{Context, Result, anyhow};
use ndarray::Array2;
use ort::{inputs, session::{Session, builder::GraphOptimizationLevel}, value::Tensor};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// Embedding dimensions for all-MiniLM-L6-v2
pub const EMBEDDING_DIM: usize = 384;

/// Maximum sequence length for the model
const MAX_SEQ_LEN: usize = 256;

/// Semantic search engine with local embeddings
pub struct SemanticEngine {
    session: Session,
    tokenizer: Tokenizer,
    /// Pre-computed principle embeddings: principle_id -> embedding
    principle_embeddings: HashMap<String, Vec<f32>>,
}

impl SemanticEngine {
    /// Initialize the semantic engine with model from cache or download
    pub fn new(model_dir: &Path) -> Result<Self> {
        // Ensure model directory exists
        std::fs::create_dir_all(model_dir)?;

        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        // Download model if not present
        if !model_path.exists() || !tokenizer_path.exists() {
            Self::download_model(model_dir)?;
        }

        // Load ONNX model
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_path)
            .context("Failed to load ONNX model")?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            session,
            tokenizer,
            principle_embeddings: HashMap::new(),
        })
    }

    /// Download the embedding model from HuggingFace
    fn download_model(model_dir: &Path) -> Result<()> {
        use hf_hub::api::sync::Api;

        println!("Downloading embedding model (all-MiniLM-L6-v2)...");
        let api = Api::new()?;
        let repo = api.model("sentence-transformers/all-MiniLM-L6-v2".to_string());

        // Download ONNX model
        let model_path = repo.get("onnx/model.onnx")?;
        std::fs::copy(&model_path, model_dir.join("model.onnx"))?;

        // Download tokenizer
        let tokenizer_path = repo.get("tokenizer.json")?;
        std::fs::copy(&tokenizer_path, model_dir.join("tokenizer.json"))?;

        println!("Model downloaded successfully!");
        Ok(())
    }

    /// Compute embedding for a single text
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| m as i64).collect();
        let token_type_ids: Vec<i64> = vec![0i64; ids.len()];

        // Truncate if necessary
        let len = ids.len().min(MAX_SEQ_LEN);
        let ids = &ids[..len];
        let attention_mask = &attention_mask[..len];
        let token_type_ids = &token_type_ids[..len];

        // Create input tensors (batch size 1)
        let input_ids = Array2::from_shape_vec((1, len), ids.to_vec())?;
        let attention = Array2::from_shape_vec((1, len), attention_mask.to_vec())?;
        let token_types = Array2::from_shape_vec((1, len), token_type_ids.to_vec())?;

        // Run inference using ort's inputs! macro with proper Tensor conversion
        let outputs = self.session.run(inputs![
            "input_ids" => Tensor::from_array(input_ids)?,
            "attention_mask" => Tensor::from_array(attention)?,
            "token_type_ids" => Tensor::from_array(token_types)?,
        ])?;

        // Extract embeddings (last_hidden_state -> mean pooling)
        let output = outputs.get("last_hidden_state")
            .or_else(|| outputs.get("token_embeddings"))
            .ok_or_else(|| anyhow!("No embedding output found"))?;

        let (shape, data) = output.try_extract_tensor::<f32>()?;

        // Mean pooling over sequence dimension
        // shape is [1, seq_len, embedding_dim], data is flat f32 slice
        // Convert shape to slice via its dimensions
        let shape_vec: Vec<i64> = shape.iter().map(|&d| d).collect();

        // Copy data before borrow ends (for borrow checker)
        let data_vec: Vec<f32> = data.to_vec();
        drop(outputs); // Release borrow

        let embedding = Self::mean_pool_flat(&data_vec, len, &shape_vec);

        // L2 normalize
        let normalized = Self::l2_normalize(&embedding);

        Ok(normalized)
    }

    /// Mean pooling over sequence dimension using flat slice
    /// tensor shape: [1, seq_len, embedding_dim]
    fn mean_pool_flat(data: &[f32], seq_len: usize, shape: &[i64]) -> Vec<f32> {
        let mut result = vec![0.0f32; EMBEDDING_DIM];

        // Get actual embedding dimension from shape (should be shape[2])
        let embed_dim = if shape.len() >= 3 { shape[2] as usize } else { EMBEDDING_DIM };

        // Data is in row-major order: [batch][seq][embed]
        // batch_size = 1, so we skip batch dimension
        for i in 0..seq_len {
            for j in 0..embed_dim.min(EMBEDDING_DIM) {
                let idx = i * embed_dim + j;
                if idx < data.len() {
                    result[j] += data[idx];
                }
            }
        }

        // Average
        for v in result.iter_mut() {
            *v /= seq_len as f32;
        }

        result
    }

    /// L2 normalize a vector
    fn l2_normalize(vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter().map(|x| x / norm).collect()
        } else {
            vec.to_vec()
        }
    }

    /// Compute cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        // Vectors are already L2 normalized, so dot product = cosine similarity
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Load pre-computed embeddings from database
    pub fn load_embeddings(&mut self, conn: &Connection) -> Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT id, embedding FROM principles WHERE embedding IS NOT NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;

        let mut count = 0;
        for row in rows {
            let (id, blob) = row?;
            // Convert bytes to f32 vector
            let embedding: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();

            if embedding.len() == EMBEDDING_DIM {
                self.principle_embeddings.insert(id, embedding);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Pre-compute and store embeddings for all principles
    pub fn compute_all_embeddings(&mut self, conn: &Connection) -> Result<usize> {
        // Get all principles without embeddings
        let mut stmt = conn.prepare(
            "SELECT id, name, description, COALESCE(application_rule, '')
             FROM principles WHERE embedding IS NULL"
        )?;

        let principles: Vec<(String, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        println!("Computing embeddings for {} principles...", principles.len());

        let mut update_stmt = conn.prepare(
            "UPDATE principles SET embedding = ?2 WHERE id = ?1"
        )?;

        let mut count = 0;
        for (id, name, description, application_rule) in principles {
            // Concatenate all text for embedding
            let text = format!("{} {} {}", name, description, application_rule);

            match self.embed(&text) {
                Ok(embedding) => {
                    // Convert to bytes
                    let bytes: Vec<u8> = embedding
                        .iter()
                        .flat_map(|f| f.to_le_bytes())
                        .collect();

                    update_stmt.execute(params![id, bytes])?;
                    count += 1;

                    if count % 50 == 0 {
                        println!("  Computed {}/{}", count, count);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to embed principle {}: {}", id, e);
                }
            }
        }

        println!("Computed {} embeddings", count);
        Ok(count)
    }

    /// Search for similar principles using semantic similarity
    pub fn search(&mut self, query: &str, top_k: usize) -> Result<Vec<SemanticMatch>> {
        let query_embedding = self.embed(query)?;

        let mut results: Vec<SemanticMatch> = self.principle_embeddings
            .iter()
            .map(|(id, emb)| {
                let similarity = Self::cosine_similarity(&query_embedding, emb);
                SemanticMatch {
                    principle_id: id.clone(),
                    similarity,
                }
            })
            .collect();

        // Sort by similarity descending
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        // Return top K
        results.truncate(top_k);
        Ok(results)
    }

    /// Hybrid search: combine semantic similarity with BM25 scores
    pub fn hybrid_search(
        &mut self,
        conn: &Connection,
        query: &str,
        top_k: usize,
        semantic_weight: f32,
    ) -> Result<Vec<HybridMatch>> {
        // 1. Get semantic matches
        let semantic_results = self.search(query, top_k * 2)?;
        let semantic_map: HashMap<String, f32> = semantic_results
            .into_iter()
            .map(|m| (m.principle_id, m.similarity))
            .collect();

        // 2. Get BM25 matches from FTS5
        let keywords: Vec<&str> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .take(15)
            .collect();

        let fts_query = keywords.join(" OR ");
        let bm25_results = Self::bm25_search(conn, &fts_query, top_k * 2)?;
        let bm25_map: HashMap<String, f32> = bm25_results
            .into_iter()
            .map(|m| (m.principle_id, m.score))
            .collect();

        // 3. Combine scores using RECIPROCAL RANK FUSION (RRF)
        // RRF is scientifically proven to be more robust than weighted averaging
        // Formula: score(d) = Σ 1 / (k + rank_i(d)), k=60 is standard
        const RRF_K: f32 = 60.0;

        // Create ranked lists
        let mut semantic_ranked: Vec<_> = semantic_map.iter().collect();
        semantic_ranked.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        let semantic_ranks: HashMap<&String, usize> = semantic_ranked
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i + 1))
            .collect();

        let mut bm25_ranked: Vec<_> = bm25_map.iter().collect();
        bm25_ranked.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
        let bm25_ranks: HashMap<&String, usize> = bm25_ranked
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i + 1))
            .collect();

        let all_ids: std::collections::HashSet<_> = semantic_map.keys()
            .chain(bm25_map.keys())
            .cloned()
            .collect();

        let mut combined: Vec<HybridMatch> = all_ids
            .into_iter()
            .map(|id| {
                let semantic = semantic_map.get(&id).copied().unwrap_or(0.0);
                let bm25 = bm25_map.get(&id).copied().unwrap_or(0.0);

                // RRF score: 1/(k+rank) for each list, sum them
                let sem_rank = semantic_ranks.get(&id).copied().unwrap_or(1000) as f32;
                let bm25_rank = bm25_ranks.get(&id).copied().unwrap_or(1000) as f32;

                // Weight semantic more heavily (70% semantic, 30% BM25 via RRF weights)
                let rrf_score = semantic_weight * (1.0 / (RRF_K + sem_rank))
                    + (1.0 - semantic_weight) * (1.0 / (RRF_K + bm25_rank));

                HybridMatch {
                    principle_id: id,
                    semantic_score: semantic,
                    bm25_score: bm25,
                    combined_score: rrf_score,
                }
            })
            .collect();

        combined.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        combined.truncate(top_k);

        Ok(combined)
    }

    /// BM25 search using FTS5
    fn bm25_search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Bm25Match>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT p.id, bm25(principles_fts) as score
            FROM principles_fts
            JOIN principles p ON principles_fts.rowid = p.rowid
            WHERE principles_fts MATCH ?1
            ORDER BY score
            LIMIT ?2
            "#
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(Bm25Match {
                    principle_id: row.get(0)?,
                    score: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}

/// Semantic search result
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub principle_id: String,
    pub similarity: f32,
}

/// BM25 search result
#[derive(Debug, Clone)]
struct Bm25Match {
    principle_id: String,
    score: f32,
}

/// Hybrid search result combining semantic and BM25
#[derive(Debug, Clone)]
pub struct HybridMatch {
    pub principle_id: String,
    pub semantic_score: f32,
    pub bm25_score: f32,
    pub combined_score: f32,
}

/// Add embedding column to schema if not exists
pub fn init_embedding_schema(conn: &Connection) -> Result<()> {
    // Check if column exists
    let has_column: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('principles') WHERE name='embedding'",
        [],
        |row| row.get(0),
    ).unwrap_or(0) > 0;

    if !has_column {
        conn.execute(
            "ALTER TABLE principles ADD COLUMN embedding BLOB",
            [],
        )?;
        println!("Added embedding column to principles table");
    }

    Ok(())
}

/// Get model directory path
pub fn get_model_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("100minds")
        .join("models")
        .join("minilm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((SemanticEngine::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((SemanticEngine::cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_l2_normalize() {
        let vec = vec![3.0, 4.0];
        let normalized = SemanticEngine::l2_normalize(&vec);

        // 3-4-5 triangle: norm should be 5
        assert!((normalized[0] - 0.6).abs() < 0.001);
        assert!((normalized[1] - 0.8).abs() < 0.001);
    }
}
