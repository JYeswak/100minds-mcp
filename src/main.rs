//! 100minds MCP Server
//!
//! Adversarial Wisdom Council - decision intelligence for AI agents.
//!
//! Run with: cargo run
//! Or via MCP: add to your claude_desktop_config.json

use anyhow::Result;
use minds_mcp::{
    counsel::CounselEngine, db, embeddings, eval, mcp, outcome, prd, provenance::Provenance,
    templates, types::*,
};
use std::path::PathBuf;

// MCP server imports would go here when mcp-server crate is available
// For now, we'll implement a simple JSON-RPC interface

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // COMMAND MODE: Handle specific commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--validate-prd" => {
                let prd_path = args.get(2).expect("Usage: --validate-prd <path>");
                return run_validate_prd(prd_path);
            }
            "--analyze-prd" => {
                let prd_path = args.get(2).expect("Usage: --analyze-prd <path>");
                let output_path = args.get(3); // Optional output path
                return run_analyze_prd(prd_path, output_path.map(|s| s.as_str()));
            }
            "--template" => {
                let question = args[2..].join(" ");
                return run_template_match(&question);
            }
            "--blind-spots" => {
                let context = args[2..].join(" ");
                return run_blind_spots(&context);
            }
            "--pre-work" => {
                let task = args[2..].join(" ");
                return run_pre_work(&task);
            }
            "--tools" => {
                // Output MCP tool definitions as JSON
                let tools = mcp::get_tools();
                println!("{}", serde_json::to_string_pretty(&tools)?);
                return Ok(());
            }
            "--stats" => {
                return run_stats();
            }
            "--benchmark" => {
                let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("all");
                return run_benchmark_cmd(subcommand, &args[3..]);
            }
            "--analyze" => {
                let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("coverage");
                return run_analyze(subcommand);
            }
            "--thompson" => {
                let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
                return run_thompson(subcommand);
            }
            "--compute-embeddings" => {
                return run_compute_embeddings();
            }
            "--outcome" => {
                return run_outcome_cmd(&args[2..]);
            }
            "--learning-stats" => {
                return run_learning_stats();
            }
            "--hybrid-search" => {
                let query = args[2..].join(" ");
                return run_hybrid_search(&query);
            }
            "counsel" => {
                // counsel <question> [--json] [--domain=X]
                let json_output = args.iter().any(|a| a == "--json");
                let domain = args
                    .iter()
                    .find(|a| a.starts_with("--domain="))
                    .map(|a| a.strip_prefix("--domain=").unwrap().to_string());
                let question: String = args[2..]
                    .iter()
                    .filter(|a| !a.starts_with("--"))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                return run_counsel_cmd(&question, domain.as_deref(), json_output);
            }
            "--serve" => {
                // HTTP server mode for swarm integration
                let port: u16 = args
                    .iter()
                    .find(|a| a.starts_with("--port="))
                    .and_then(|a| a.strip_prefix("--port=").and_then(|p| p.parse().ok()))
                    .unwrap_or(3100);
                return run_http_server(port).await;
            }
            "--sync-posteriors" => {
                // Output posteriors as JSON for swarm sync
                return run_sync_posteriors();
            }
            s if !s.starts_with("--") => {
                // One-shot counsel (legacy)
                let question = args[1..].join(" ");
                return run_oneshot(&question);
            }
            _ => {}
        }
    }

    // REPL MODE: Only if explicitly requested or no args
    if args.contains(&"--repl".to_string()) || args.len() == 1 {
        tracing_subscriber::fmt::init();
        let data_dir = get_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("wisdom.db");
        let conn = db::init_db(&db_path)?;
        tracing::info!("Database initialized at {:?}", db_path);
        let key_path = data_dir.join("agent.key");
        let provenance = Provenance::init(&key_path)?;
        tracing::info!(
            "Provenance initialized, pubkey: {}",
            provenance.public_key_hex()
        );
        run_cli_mode(&conn, &provenance).await?;
    }

    Ok(())
}

/// Analyze a PRD and output enhanced version with 100minds metadata
fn run_analyze_prd(prd_path: &str, output_path: Option<&str>) -> Result<()> {
    let prd_content = std::fs::read_to_string(prd_path)?;
    let mut prd_doc: prd::Prd = prd::from_json(&prd_content)?;

    // Analyze with 100minds
    let metadata = prd::analyze_prd(&mut prd_doc);
    prd_doc.minds_metadata = Some(metadata.clone());

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS PRD ANALYSIS                                    │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    println!("PRD: {} ({})", prd_doc.title, prd_doc.id);
    println!("Stories: {}", prd_doc.stories.len());
    println!();

    // Score
    let score_bar = "█".repeat((metadata.validation_score / 10.0) as usize);
    let empty_bar = "░".repeat(10 - (metadata.validation_score / 10.0) as usize);
    let status = if metadata.validation_score >= 70.0 {
        "✅ GOOD"
    } else {
        "⚠️ NEEDS WORK"
    };
    println!(
        "Score: [{score_bar}{empty_bar}] {:.0}/100  {status}",
        metadata.validation_score
    );
    println!();

    // Principles applied
    if !metadata.principles_applied.is_empty() {
        println!(
            "📚 Principles Applied: {}",
            metadata.principles_applied.join(", ")
        );
        println!();
    }

    // Warnings
    if !metadata.warnings.is_empty() {
        println!("⚠️ WARNINGS:");
        for w in &metadata.warnings {
            println!("   • {}", w);
        }
        println!();
    }

    // Split recommendation
    if let Some(split) = &metadata.split_recommendation {
        if split.should_split {
            println!("✂️ SPLIT RECOMMENDATION:");
            println!("   {}", split.reason);
            for suggested in &split.suggested_prds {
                println!("   📁 {}", suggested.title);
                println!("      Stories: {}", suggested.story_ids.join(", "));
                println!("      Reason: {}", suggested.rationale);
            }
            println!();
        } else {
            println!("✅ PRD is well-scoped (no split needed)");
            println!();
        }
    }

    // Output enhanced PRD if requested
    if let Some(output_path) = output_path {
        let enhanced_json = prd::to_json(&prd_doc)?;
        std::fs::write(output_path, enhanced_json)?;
        println!("📄 Enhanced PRD saved to: {}", output_path);
    }

    Ok(())
}