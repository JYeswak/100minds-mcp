//! Import thinker data from 100minds research output
//!
//! Usage: cargo run --bin import -- /path/to/100minds-*/output

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ThinkerProfile {
    name: String,
    #[serde(alias = "bio")]
    background: Option<String>,
    // Handle multiple field names: principles, key_principles, keyPrinciples
    #[serde(alias = "key_principles", alias = "keyPrinciples")]
    principles: Option<Vec<PrincipleVariant>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PrincipleVariant {
    // Full object with name/principle and description
    Object {
        #[serde(alias = "principle")]
        name: String,
        description: String,
    },
    // Just a string (common in some profiles)
    String(String),
}

impl PrincipleVariant {
    fn to_name_desc(&self) -> (String, String) {
        match self {
            PrincipleVariant::Object { name, description } => (name.clone(), description.clone()),
            PrincipleVariant::String(s) => {
                // Try to split on colon for "Name: Description" format
                if let Some((name, desc)) = s.split_once(':') {
                    (name.trim().to_string(), desc.trim().to_string())
                } else {
                    // Use first few words as name, rest as description
                    let words: Vec<&str> = s.split_whitespace().collect();
                    let name = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
                    (name, s.clone())
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <research-output-dirs...>", args[0]);
        eprintln!(
            "Example: {} ~/Desktop/Projects/100minds-ai-ml/output",
            args[0]
        );
        std::process::exit(1);
    }

    // Get data directory
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("100minds");
    fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("wisdom.db");
    println!("Opening database at {:?}", db_path);

    let conn = Connection::open(&db_path)?;

    let mut total_thinkers = 0;
    let mut total_principles = 0;

    // Process each output directory
    for arg in &args[1..] {
        let output_dir = PathBuf::from(arg);
        if !output_dir.exists() {
            eprintln!("Warning: {} does not exist, skipping", arg);
            continue;
        }

        println!("\nProcessing: {:?}", output_dir);

        let (t, p) = import_directory(&conn, &output_dir)?;
        total_thinkers += t;
        total_principles += p;
    }

    println!("\n========================================");
    println!("Import complete!");
    println!("  Thinkers: {}", total_thinkers);
    println!("  Principles: {}", total_principles);
    println!("========================================");

    Ok(())
}

fn import_directory(conn: &Connection, dir: &Path) -> Result<(usize, usize)> {
    let mut thinkers = 0;
    let mut principles = 0;

    // First: Try operator format (flat JSON files like TIM_FERRISS--FEAR_SETTING.json)
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.ends_with(".json") && n.contains("--") && n != "CLAUDE.md")
                .unwrap_or(false)
        })
    {
        let path = entry.path();
        match import_operator(conn, path) {
            Ok((t, p)) => {
                thinkers += t;
                principles += p;
                if p > 0 {
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                    println!("  ✓ {} (operator)", fname);
                }
            }
            Err(e) => {
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                eprintln!("  ✗ {}: {}", fname, e);
            }
        }
    }

    // Second: Try profile.json format (nested directories)
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "profile.json")
    {
        let path = entry.path();

        // Determine domain from path structure
        // Expected: output/<domain>/<thinker>/profile.json
        let parts: Vec<_> = path.components().collect();
        let domain = parts
            .iter()
            .rev()
            .nth(2)
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("unknown");

        // Thinker ID from directory name
        let thinker_id = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        match import_profile(conn, path, domain, thinker_id) {
            Ok((t, p)) => {
                thinkers += t;
                principles += p;
                println!("  ✓ {} ({} principles)", thinker_id, p);
            }
            Err(e) => {
                eprintln!("  ✗ {}: {}", thinker_id, e);
            }
        }
    }

    Ok((thinkers, principles))
}

/// Import a V2 operator file (like TIM_FERRISS--FEAR_SETTING.json)
fn import_operator(conn: &Connection, path: &Path) -> Result<(usize, usize)> {
    let content = fs::read_to_string(path)?;
    let op: serde_json::Value = serde_json::from_str(&content)?;

    // Extract fields from operator format
    let thinker_slug = op
        .get("thinker_slug")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing thinker_slug"))?;

    let name = op
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing name"))?;

    let description = op.get("description").and_then(|v| v.as_str()).unwrap_or("");

    // Get extended description if available
    let extended = op
        .get("extended_description")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let full_description = if extended.is_empty() {
        description.to_string()
    } else {
        format!("{} {}", description, extended)
    };

    // Derive thinker name from slug
    let thinker_name = thinker_slug
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Ensure thinker exists
    conn.execute(
        "INSERT OR IGNORE INTO thinkers (id, name, domain, background)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            thinker_slug,
            thinker_name,
            "entrepreneurship", // Default domain for operators
            "",
        ],
    )?;

    // Create principle ID from operator ID
    let principle_id = op.get("id").and_then(|v| v.as_str()).unwrap_or(name);

    // Extract domain tags from when_to_use.contexts if available
    let domain_tags = if let Some(when_to_use) = op.get("when_to_use") {
        if let Some(keywords) = when_to_use.get("keywords").and_then(|v| v.as_array()) {
            let kw_strings: Vec<String> = keywords
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .take(10)
                .collect();
            serde_json::to_string(&kw_strings)?
        } else {
            "[]".to_string()
        }
    } else {
        "[]".to_string()
    };

    // Insert the principle
    conn.execute(
        "INSERT OR REPLACE INTO principles
         (id, thinker_id, name, description, domain_tags, base_confidence, learned_confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, 0.7, 0.7)",
        params![
            principle_id,
            thinker_slug,
            name,
            full_description,
            domain_tags,
        ],
    )?;

    Ok((0, 1)) // 0 new thinkers (may already exist), 1 principle
}

fn import_profile(
    conn: &Connection,
    path: &Path,
    domain: &str,
    thinker_id: &str,
) -> Result<(usize, usize)> {
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;

    // Try to parse - handle both object format and raw JSON
    let profile: ThinkerProfile = if content.trim().starts_with('{') {
        serde_json::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))?
    } else {
        // Might be partial/malformed, try to extract name at minimum
        ThinkerProfile {
            name: thinker_id.replace('-', " "),
            background: Some(content.clone()),
            principles: None,
        }
    };

    // Insert thinker
    conn.execute(
        "INSERT OR REPLACE INTO thinkers (id, name, domain, background, profile_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            thinker_id,
            profile.name,
            domain,
            profile.background,
            content,
        ],
    )?;

    // Import principles
    let principles = profile.principles.unwrap_or_default();
    let mut principle_count = 0;

    for (i, principle_variant) in principles.iter().enumerate() {
        let (name, description) = principle_variant.to_name_desc();
        let principle_id = format!("{}-{}", thinker_id, i + 1);
        let domain_tags = serde_json::to_string(&vec![domain])?;

        conn.execute(
            "INSERT OR REPLACE INTO principles
             (id, thinker_id, name, description, domain_tags, base_confidence, learned_confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, 0.5, 0.5)",
            params![principle_id, thinker_id, name, description, domain_tags,],
        )?;

        principle_count += 1;
    }

    Ok((1, principle_count))
}
