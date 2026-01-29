//! 100minds MCP Server
//!
//! Adversarial Wisdom Council - decision intelligence for AI agents.
//!
//! Run with: cargo run
//! Or via MCP: add to your claude_desktop_config.json

use anyhow::Result;
use minds_mcp::{
    counsel::CounselEngine,
    db, embeddings, eval, mcp, outcome, prd, provenance::Provenance,
    templates,
    types::*,
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
                let domain = args.iter()
                    .find(|a| a.starts_with("--domain="))
                    .map(|a| a.strip_prefix("--domain=").unwrap().to_string());
                let question: String = args[2..].iter()
                    .filter(|a| !a.starts_with("--"))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                return run_counsel_cmd(&question, domain.as_deref(), json_output);
            }
            "--serve" => {
                // HTTP server mode for swarm integration
                let port: u16 = args.iter()
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
        tracing::info!("Provenance initialized, pubkey: {}", provenance.public_key_hex());
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
    let status = if metadata.validation_score >= 70.0 { "✅ GOOD" } else { "⚠️ NEEDS WORK" };
    println!("Score: [{}{score_bar}{empty_bar}] {:.0}/100  {}", "", metadata.validation_score, status);
    println!();

    // Principles applied
    if !metadata.principles_applied.is_empty() {
        println!("📚 Principles Applied: {}", metadata.principles_applied.join(", "));
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

    // Scope analysis
    println!("📋 SCOPE ANALYSIS:");
    println!("   In Scope ({}):", metadata.scope_analysis.in_scope.len());
    for item in &metadata.scope_analysis.in_scope {
        println!("      ✓ {}", item);
    }

    if !metadata.scope_analysis.out_of_scope.is_empty() {
        println!("   Out of Scope ({}):", metadata.scope_analysis.out_of_scope.len());
        for item in &metadata.scope_analysis.out_of_scope {
            println!("      ✗ {}", item);
        }
    }

    if !metadata.scope_analysis.deferred.is_empty() {
        println!("   Deferred ({}):", metadata.scope_analysis.deferred.len());
        for item in &metadata.scope_analysis.deferred {
            println!("      ⏸ {} → {}", item.item, item.suggested_prd.as_deref().unwrap_or("later"));
            println!("         Reason: {}", item.reason);
        }
    }
    println!();

    // Output enhanced PRD JSON
    if let Some(out_path) = output_path {
        let json = prd::to_json(&prd_doc)?;
        std::fs::write(out_path, &json)?;
        println!("📄 Enhanced PRD written to: {}", out_path);
    } else {
        println!("─────────────────────────────────────────────────────────────");
        println!("📄 ENHANCED PRD JSON (use --analyze-prd <input> <output> to save):");
        println!("─────────────────────────────────────────────────────────────");
        println!("{}", prd::to_json(&prd_doc)?);
    }

    Ok(())
}

/// Validate a PRD against 100minds principles
fn run_validate_prd(prd_path: &str) -> Result<()> {
    let prd_content = std::fs::read_to_string(prd_path)?;
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    let result = mcp::validate_prd(&conn, &prd_content)?;

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS PRD VALIDATION                                  │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Score with visual bar
    let score_bar = "█".repeat((result.score / 10.0) as usize);
    let empty_bar = "░".repeat(10 - (result.score / 10.0) as usize);
    let status = if result.valid { "✅ PASSED" } else { "❌ FAILED" };

    println!("Score: [{}{score_bar}{empty_bar}] {:.0}/100  {}", "", result.score, status);
    println!();

    // Warnings by severity
    let errors: Vec<_> = result.warnings.iter().filter(|w| w.severity == "error").collect();
    let warnings: Vec<_> = result.warnings.iter().filter(|w| w.severity == "warning").collect();
    let infos: Vec<_> = result.warnings.iter().filter(|w| w.severity == "info").collect();

    if !errors.is_empty() {
        println!("🔴 ERRORS ({}):", errors.len());
        for w in errors {
            println!("   • {} ({}): {}", w.principle, w.thinker, w.message);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("🟡 WARNINGS ({}):", warnings.len());
        for w in warnings {
            println!("   • {} ({}): {}", w.principle, w.thinker, w.message);
        }
        println!();
    }

    if !infos.is_empty() {
        println!("🔵 INFO ({}):", infos.len());
        for w in infos {
            println!("   • {} ({}): {}", w.principle, w.thinker, w.message);
        }
        println!();
    }

    // Suggestions
    if !result.suggestions.is_empty() {
        println!("💡 SUGGESTIONS:");
        for s in &result.suggestions {
            println!("   • {} ({}): {}", s.principle, s.thinker, s.suggestion);
        }
        println!();
    }

    // Blind spots
    if !result.blind_spots_to_check.is_empty() {
        println!("👁️ BLIND SPOTS TO CHECK:");
        for bs in &result.blind_spots_to_check {
            println!("   • {}", bs);
        }
        println!();
    }

    // Principles applied
    println!("📚 Principles Applied: {}", result.principles_applied.join(", "));

    Ok(())
}

/// Match decision to templates
fn run_template_match(question: &str) -> Result<()> {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS DECISION TEMPLATES                              │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    let matches = mcp::get_matching_templates(question);

    if matches.is_empty() {
        println!("No matching templates found for: {}", question);
        println!("\nAvailable templates:");
        for t in templates::get_templates() {
            println!("  • {} - {}", t.id, t.description);
        }
        return Ok(());
    }

    for (i, m) in matches.iter().enumerate() {
        println!("{}. {} (match score: {:.1})", i + 1, m.template.name, m.match_score);
        println!("   {}", m.template.description);
        println!();

        // Print decision tree
        println!("   DECISION TREE:");
        print_decision_tree_node(&m.template.tree, "   ");
        println!();

        // Print synergies
        if !m.template.synergies.is_empty() {
            println!("   SYNERGIES:");
            for s in &m.template.synergies {
                println!("   • {} + {} = {}", s.principles.join(" + "), s.thinkers.join(", "), s.combined_power);
            }
            println!();
        }

        // Print blind spots
        if !m.template.blind_spots.is_empty() {
            println!("   BLIND SPOTS:");
            for bs in &m.template.blind_spots {
                println!("   • [{:?}] {}: {}", bs.severity, bs.name, bs.check_question);
            }
            println!();
        }

        // Print anti-patterns
        if !m.template.anti_patterns.is_empty() {
            println!("   ANTI-PATTERNS TO AVOID:");
            for ap in &m.template.anti_patterns {
                println!("   • {} ({}): {}", ap.name, ap.source_thinker, ap.description);
            }
            println!();
        }
    }

    Ok(())
}

fn print_decision_tree_node(tree: &templates::DecisionTree, prefix: &str) {
    println!("{}❓ {}", prefix, tree.question);
    if let Some(help) = &tree.help_text {
        println!("{}   ({})", prefix, help);
    }

    for (i, opt) in tree.options.iter().enumerate() {
        let branch = if i == tree.options.len() - 1 { "└" } else { "├" };
        println!("{}{}─ {} - {}", prefix, branch, opt.label, opt.description);

        if let Some(rec) = &opt.recommendation {
            for line in wrap_lines(rec, 60) {
                println!("{}   ➜ {}", prefix, line);
            }
        }

        if let Some(next) = &opt.next {
            print_decision_tree_node(next, &format!("{}   ", prefix));
        }
    }
}

/// Check blind spots for a decision
fn run_blind_spots(context: &str) -> Result<()> {
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS BLIND SPOT ANALYSIS                             │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    let analysis = mcp::check_blind_spots(context, None);

    println!("Context: {}", context);
    println!("Critical blind spots: {}\n", analysis.critical_count);

    for bs in &analysis.blind_spots {
        let icon = match bs.severity.as_str() {
            "Critical" => "🔴",
            "High" => "🟠",
            "Medium" => "🟡",
            _ => "🔵",
        };

        println!("{} [{}] {}", icon, bs.severity, bs.name);
        println!("   {}", bs.description);
        println!("   ❓ CHECK: {}", bs.check_question);
        println!("   (from: {})", bs.source_template);
        println!();
    }

    Ok(())
}

/// Get pre-work context for a task
fn run_pre_work(task: &str) -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    let context = mcp::get_pre_work_context(&conn, task, task, Some("feature"))?;

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS PRE-WORK CONTEXT                                │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    println!("Task: {}", context.task_title);
    println!("Type: {}\n", context.task_type);

    if !context.relevant_principles.is_empty() {
        println!("📚 RELEVANT PRINCIPLES:");
        for p in &context.relevant_principles {
            println!("   • {} ({})", p.name, p.thinker);
            println!("     {}", truncate_str(&p.description, 60));
            println!("     ⚡ {}", p.action);
            println!();
        }
    }

    println!("👁️ BLIND SPOTS:");
    for bs in &context.blind_spots {
        println!("   • {}", bs);
    }
    println!();

    println!("🚫 ANTI-PATTERNS TO AVOID:");
    for ap in &context.anti_patterns_to_avoid {
        println!("   • {}", ap);
    }
    println!();

    println!("❓ KEY QUESTIONS:");
    for q in &context.key_questions {
        println!("   • {}", q);
    }

    Ok(())
}

/// Show wisdom statistics
fn run_stats() -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS WISDOM STATISTICS                               │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Count thinkers
    let thinker_count: i64 = conn.query_row("SELECT COUNT(*) FROM thinkers", [], |row| row.get(0))?;
    println!("Thinkers: {}", thinker_count);

    // Count principles
    let principle_count: i64 = conn.query_row("SELECT COUNT(*) FROM principles", [], |row| row.get(0))?;
    println!("Principles: {}", principle_count);

    // Count decisions
    let decision_count: i64 = conn.query_row("SELECT COUNT(*) FROM decisions", [], |row| row.get(0))?;
    println!("Decisions recorded: {}", decision_count);

    // Count outcomes
    let outcome_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decisions WHERE outcome_success IS NOT NULL",
        [],
        |row| row.get(0)
    )?;
    println!("Outcomes recorded: {}", outcome_count);

    // Templates
    let template_count = templates::get_templates().len();
    println!("Decision templates: {}", template_count);

    // Top principles by confidence
    println!("\n📈 TOP PRINCIPLES BY CONFIDENCE:");
    let mut stmt = conn.prepare(
        "SELECT name, learned_confidence FROM principles ORDER BY learned_confidence DESC LIMIT 5"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (name, conf) = row?;
        let bar = "█".repeat((conf * 10.0) as usize);
        println!("   [{:.<10}] {:.0}% {}", bar, conf * 100.0, name);
    }

    Ok(())
}

/// Fast one-shot query - no logging, minimal overhead
fn run_oneshot(question: &str) -> Result<()> {
    use std::time::Instant;
    let start = Instant::now();

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;
    let key_path = data_dir.join("agent.key");
    let provenance = Provenance::init(&key_path)?;

    let engine = CounselEngine::new(&conn, &provenance);
    let request = CounselRequest {
        question: question.to_string(),
        context: CounselContext::default(),
    };

    match engine.counsel(&request) {
        Ok(response) => print_decision_tree(&response, start.elapsed()),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}

// ============================================================================
// SWARM INTEGRATION CLI COMMANDS
// ============================================================================

/// Counsel command with JSON output support for swarm integration
fn run_counsel_cmd(question: &str, domain: Option<&str>, json_output: bool) -> Result<()> {
    use std::time::Instant;
    let start = Instant::now();

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;
    let key_path = data_dir.join("agent.key");
    let provenance = Provenance::init(&key_path)?;

    let engine = CounselEngine::new(&conn, &provenance);
    let request = CounselRequest {
        question: question.to_string(),
        context: CounselContext {
            domain: domain.map(String::from),
            ..Default::default()
        },
    };

    match engine.counsel(&request) {
        Ok(response) => {
            if json_output {
                // JSON output for swarm integration
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                print_decision_tree(&response, start.elapsed());
            }
        }
        Err(e) => {
            if json_output {
                let error = serde_json::json!({"error": e.to_string()});
                println!("{}", serde_json::to_string_pretty(&error)?);
            } else {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

/// Sync posteriors command for swarm integration
fn run_sync_posteriors() -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    // Initialize Thompson schema
    eval::thompson::init_thompson_schema(&conn)?;

    let response = outcome::sync_posteriors(&conn, None, None)?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

/// HTTP server mode for swarm integration
async fn run_http_server(port: u16) -> Result<()> {
    use std::net::TcpListener;

    eprintln!("🚀 100minds MCP Server starting on port {}...", port);

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let key_path = data_dir.join("agent.key");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    eprintln!("✅ Listening on http://localhost:{}/mcp", port);

    for stream in listener.incoming() {
        let stream = stream?;
        let db_path = db_path.clone();
        let key_path = key_path.clone();

        // Handle each connection
        std::thread::spawn(move || {
            if let Err(e) = handle_http_request(stream, &db_path, &key_path) {
                eprintln!("Request error: {}", e);
            }
        });
    }

    Ok(())
}

fn handle_http_request(
    mut stream: std::net::TcpStream,
    db_path: &std::path::Path,
    key_path: &std::path::Path,
) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Read headers
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        if header.trim().is_empty() {
            break;
        }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header.split(':').nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }

    // Read body
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        std::io::Read::read_exact(&mut reader, &mut body)?;
    }

    // Parse JSON-RPC request
    let body_str = String::from_utf8_lossy(&body);
    let json_req: serde_json::Value = serde_json::from_str(&body_str)
        .unwrap_or(serde_json::json!({}));

    let method = json_req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = json_req.get("params").cloned().unwrap_or(serde_json::json!({}));
    let id = json_req.get("id").cloned().unwrap_or(serde_json::json!(1));

    // Route to handler
    let conn = db::init_db(db_path)?;
    let provenance = Provenance::init(key_path)?;

    let result = match method {
        "counsel" | "tools/call" => {
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("counsel");
            match tool_name {
                "counsel" => handle_counsel_tool(&conn, &provenance, &params),
                "record_outcome" => handle_record_outcome_tool(&conn, &params),
                "sync_posteriors" => handle_sync_posteriors_tool(&conn, &params),
                "counterfactual_sim" => handle_counterfactual_sim_tool(&conn, &provenance, &params),
                _ => Ok(serde_json::json!({"error": format!("Unknown tool: {}", tool_name)})),
            }
        }
        _ => Ok(serde_json::json!({"error": format!("Unknown method: {}", method)})),
    };

    let response_body = match result {
        Ok(r) => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": r
        }),
        Err(e) => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": e.to_string()}
        }),
    };

    let response_str = serde_json::to_string(&response_body)?;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        response_str.len(),
        response_str
    );

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn handle_counsel_tool(
    conn: &rusqlite::Connection,
    provenance: &Provenance,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let args = params.get("arguments").unwrap_or(params);
    let question = args.get("question").and_then(|q| q.as_str()).unwrap_or("");
    let domain = args.get("domain").and_then(|d| d.as_str());

    let engine = CounselEngine::new(conn, provenance);
    let request = CounselRequest {
        question: question.to_string(),
        context: CounselContext {
            domain: domain.map(String::from),
            ..Default::default()
        },
    };

    let response = engine.counsel(&request)?;
    Ok(serde_json::to_value(&response)?)
}

fn handle_record_outcome_tool(
    conn: &rusqlite::Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let args = params.get("arguments").unwrap_or(params);
    let decision_id = args.get("decision_id").and_then(|d| d.as_str()).unwrap_or("");
    let success = args.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
    let notes = args.get("notes").and_then(|n| n.as_str());
    let principle_ids: Vec<String> = args.get("principle_ids")
        .and_then(|p| p.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let domain = args.get("domain").and_then(|d| d.as_str());
    let confidence_score = args.get("confidence_score").and_then(|c| c.as_f64());
    let failure_stage = args.get("failure_stage").and_then(|f| f.as_str());

    let request = RecordOutcomeRequest {
        decision_id: decision_id.to_string(),
        success,
        notes: notes.map(String::from),
        principle_ids,
        domain: domain.map(String::from),
        confidence_score,
        failure_stage: failure_stage.map(String::from),
    };

    let result = outcome::record_outcome_v2(conn, &request)?;
    Ok(serde_json::to_value(&result)?)
}

fn handle_sync_posteriors_tool(
    conn: &rusqlite::Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let args = params.get("arguments").unwrap_or(params);
    let since_ts = args.get("since_ts").and_then(|t| t.as_i64());
    let domain = args.get("domain").and_then(|d| d.as_str());

    eval::thompson::init_thompson_schema(conn)?;
    let response = outcome::sync_posteriors(conn, since_ts, domain)?;
    Ok(serde_json::to_value(&response)?)
}

fn handle_counterfactual_sim_tool(
    conn: &rusqlite::Connection,
    provenance: &Provenance,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let args = params.get("arguments").unwrap_or(params);
    let question = args.get("question").and_then(|q| q.as_str()).unwrap_or("");
    let domain = args.get("domain").and_then(|d| d.as_str());
    let excluded_principles: Vec<String> = args.get("excluded_principles")
        .and_then(|p| p.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let engine = CounselEngine::new(conn, provenance);
    let request = CounselRequest {
        question: question.to_string(),
        context: CounselContext {
            domain: domain.map(String::from),
            ..Default::default()
        },
    };

    let response = engine.counterfactual_counsel(&request, &excluded_principles)?;
    Ok(serde_json::to_value(&response)?)
}

/// Print as a decision tree with reasoning chains
fn print_decision_tree(response: &CounselResponse, elapsed: std::time::Duration) {
    // Header with timing
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 100MINDS DECISION INTELLIGENCE   [{:>6.1}ms]              │", elapsed.as_secs_f64() * 1000.0);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();
    println!("📋 {}", response.question);
    println!();

    // Decision tree format
    println!("┌─ IF YOU PROCEED ────────────────────────────────────────────");
    for position in response.positions.iter().filter(|p| matches!(p.stance, Stance::For | Stance::Synthesize)) {
        print_tree_node(position, "│  ");
    }
    println!("│");

    println!("├─ WATCH OUT FOR ──────────────────────────────────────────────");
    for position in response.positions.iter().filter(|p| matches!(p.stance, Stance::Against)) {
        print_tree_node(position, "│  ");
    }
    println!("│");

    println!("└─ BEFORE DECIDING ─────────────────────────────────────────────");
    print_challenge_node(&response.challenge);
    println!();

    // Provenance footer
    println!("─────────────────────────────────────────────────────────────────");
    println!("Decision #{} │ Chain: {}",
        &response.decision_id[..8],
        response.provenance.previous_hash.as_ref()
            .map(|h| format!("{}→{}", &h[..6], &response.provenance.content_hash[..6]))
            .unwrap_or_else(|| format!("genesis→{}", &response.provenance.content_hash[..6]))
    );
}

fn print_tree_node(position: &CounselPosition, prefix: &str) {
    // Extract the ACTION from the argument
    let (principle, action) = if let Some(idx) = position.argument.find("→ ACTION:") {
        let (p, a) = position.argument.split_at(idx);
        (p.trim(), a.trim_start_matches("→ ").trim())
    } else {
        (position.argument.as_str(), "Apply this principle now.")
    };

    // Confidence indicator
    let conf_bar = match (position.confidence * 10.0) as u8 {
        0..=3 => "░░░",
        4..=6 => "▒▒░",
        7..=8 => "▓▒░",
        _ => "███",
    };

    println!("{}┌─ {} says: [{}]", prefix, position.thinker, conf_bar);

    // Wrap principle text
    for line in wrap_lines(principle, 55) {
        println!("{}│  {}", prefix, line);
    }

    // Action in bold-like format
    println!("{}│", prefix);
    println!("{}│  ⚡ {}", prefix, action);

    // Falsification
    if let Some(f) = &position.falsifiable_if {
        println!("{}│  ⚠️  Skip if: {}", prefix, truncate_str(f, 50));
    }
    println!("{}└", prefix);
}

fn print_challenge_node(challenge: &CounselPosition) {
    println!("   🔍 Devil's Advocate:");
    for line in wrap_lines(&challenge.argument, 58) {
        println!("      {}", line);
    }
}

fn wrap_lines(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in s.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())-3])
    }
}

fn get_data_dir() -> Result<PathBuf> {
    // Use XDG data dir on Linux, ~/Library/Application Support on macOS
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(base.join("100minds"))
}

async fn run_cli_mode(
    conn: &rusqlite::Connection,
    provenance: &Provenance,
) -> Result<()> {
    use std::io::{self, BufRead, Write};

    let engine = CounselEngine::new(conn, provenance);

    println!("100minds Adversarial Wisdom Council");
    println!("====================================");
    println!("Enter a decision question, or 'quit' to exit.\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line == "quit" || line == "exit" {
            break;
        }

        // Parse command
        if line.starts_with("/") {
            handle_command(&engine, line)?;
        } else {
            // Treat as counsel request
            let request = CounselRequest {
                question: line.to_string(),
                context: CounselContext::default(),
            };

            match engine.counsel(&request) {
                Ok(response) => print_counsel_response(&response),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }

    Ok(())
}

fn handle_command(engine: &CounselEngine, line: &str) -> Result<()> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).unwrap_or(&"");

    match cmd {
        "/outcome" => {
            // /outcome <decision_id> <success|fail> [notes]
            let parts: Vec<&str> = arg.splitn(3, ' ').collect();
            if parts.len() < 2 {
                println!("Usage: /outcome <decision_id> <success|fail> [notes]");
                return Ok(());
            }

            let decision_id = parts[0];
            let success = parts[1] == "success" || parts[1] == "true";
            let notes = parts.get(2).map(|s| s.to_string());

            let request = RecordOutcomeRequest {
                decision_id: decision_id.to_string(),
                success,
                notes,
                principle_ids: vec![],
                domain: None,
                confidence_score: None,
                failure_stage: None,
            };

            engine.record_outcome(&request)?;
            println!("Outcome recorded for decision {}", decision_id);
        }
        "/help" => {
            println!("Commands:");
            println!("  <question>           Ask for adversarial counsel");
            println!("  /outcome <id> <s|f>  Record outcome (success/fail)");
            println!("  /help                Show this help");
            println!("  quit                 Exit");
        }
        _ => {
            println!("Unknown command: {}. Try /help", cmd);
        }
    }

    Ok(())
}

fn print_counsel_response(response: &CounselResponse) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ ADVERSARIAL COUNSEL                                       ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║ Question: {}", truncate(&response.question, 50));
    println!("║ Decision ID: {}", response.decision_id);
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    for position in &response.positions {
        println!("{} {} ({})", position.stance.emoji(), position.thinker, position.stance.name());
        println!("   Confidence: {:.0}%", position.confidence * 100.0);
        println!("   {}", wrap_text(&position.argument, 60, "   "));
        if let Some(falsifiable) = &position.falsifiable_if {
            println!("   ⚠️  Wrong if: {}", falsifiable);
        }
        println!();
    }

    // Print challenge
    println!("{} CHALLENGE", response.challenge.stance.emoji());
    println!("   {}", wrap_text(&response.challenge.argument, 60, "   "));
    println!();

    // Print provenance
    println!("─────────────────────────────────────────────────────────────");
    println!("Provenance: {}", &response.provenance.content_hash[..16]);
    if let Some(prev) = &response.provenance.previous_hash {
        println!("Chain: ...{} → {}", &prev[..8], &response.provenance.content_hash[..8]);
    }
    println!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max-3])
    }
}

fn wrap_text(s: &str, width: usize, prefix: &str) -> String {
    let mut result = String::new();
    let mut line = String::new();

    for word in s.split_whitespace() {
        if line.len() + word.len() + 1 > width {
            if !result.is_empty() {
                result.push('\n');
                result.push_str(prefix);
            }
            result.push_str(&line);
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }

    if !line.is_empty() {
        if !result.is_empty() {
            result.push('\n');
            result.push_str(prefix);
        }
        result.push_str(&line);
    }

    result
}

// ============================================================================
// Benchmark / Evaluation Commands
// ============================================================================

/// Run benchmark suite (scenarios, monte-carlo, coverage, judge)
fn run_benchmark_cmd(subcommand: &str, args: &[String]) -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;
    let key_path = data_dir.join("agent.key");
    let provenance = Provenance::init(&key_path)?;

    match subcommand {
        "scenarios" => {
            let scenario_dir = args.first()
                .map(|s| PathBuf::from(s))
                .unwrap_or_else(|| data_dir.join("scenarios"));

            println!("Loading scenarios from {:?}...", scenario_dir);

            let scenarios = eval::scenarios::load_all_scenarios(&scenario_dir)?;
            if scenarios.is_empty() {
                println!("No scenarios found. Create JSON files in {:?}", scenario_dir);
                println!("\nExample scenario format:");
                println!(r#"{{
  "id": "arch-001",
  "category": "architecture",
  "question": "Should we use microservices?",
  "expected_principles": ["Start with Monolith", "Distributed Systems Fallacies"],
  "expected_thinkers": ["Sam Newman", "Martin Fowler"],
  "anti_principles": ["Always Use Microservices"],
  "difficulty": 3
}}"#);
                return Ok(());
            }

            println!("Running {} scenarios...", scenarios.len());
            let results = eval::scenarios::run_benchmark(&conn, &provenance, &scenarios)?;

            // Print results
            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 📊 SCENARIO BENCHMARK RESULTS                               │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            println!("Total scenarios: {}", results.total_scenarios);
            println!();

            // Aggregate metrics
            println!("AGGREGATE METRICS:");
            for k in [1, 3, 5] {
                if let Some(&p) = results.aggregate.precision_at_k.get(&k) {
                    println!("   P@{}: {:.1}%", k, p * 100.0);
                }
            }
            println!("   Recall: {:.1}%", results.aggregate.recall * 100.0);
            println!("   NDCG: {:.3}", results.aggregate.ndcg);
            println!("   Anti-principle rate: {:.1}%", results.aggregate.anti_principle_rate * 100.0);
            println!("   Avg latency: {}ms", results.aggregate.latency_ms);
            println!();

            // By category
            println!("BY CATEGORY:");
            for (cat, metrics) in &results.by_category {
                let p3 = metrics.precision_at_k.get(&3).unwrap_or(&0.0);
                println!("   {:20} P@3: {:.0}%  Recall: {:.0}%", cat, p3 * 100.0, metrics.recall * 100.0);
            }
            println!();

            // Worst performers
            if !results.worst_performers.is_empty() {
                println!("WORST PERFORMERS:");
                for result in results.worst_performers.iter().take(5) {
                    let p3 = result.metrics.precision_at_k.get(&3).unwrap_or(&0.0);
                    println!("   [P@3: {:.0}%] {} - {}", p3 * 100.0, result.scenario_id, result.question);
                }
            }
        }

        "monte-carlo" => {
            let num_sims = args.first()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000);

            println!("Running {} Monte Carlo simulations...", num_sims);

            let config = eval::monte_carlo::MonteCarloConfig {
                num_simulations: num_sims,
                ..Default::default()
            };

            let results = eval::monte_carlo::run_simulation(&conn, &provenance, &config)?;

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 🎲 MONTE CARLO RESULTS                                      │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            println!("Simulations: {}", results.num_simulations);
            println!("Selection variance: {:.3}", results.selection_variance);
            println!("95% CI: [{:.2}, {:.2}]",
                     results.confidence_interval_95.0,
                     results.confidence_interval_95.1);
            println!("Tail risk (<50%): {:.1}%", results.tail_risk * 100.0);
            println!();

            // Top selected
            println!("TOP 10 MOST SELECTED:");
            let mut sorted: Vec<_> = results.principle_selection_rates.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            for (name, rate) in sorted.iter().take(10) {
                println!("   {:.1}% - {}", *rate * 100.0, name);
            }
            println!();

            // Outcome distribution
            println!("SIMULATED OUTCOMES:");
            println!("   Success: {:.1}%", results.simulated_outcomes.success_rate * 100.0);
            println!("   Partial: {:.1}%", results.simulated_outcomes.partial_success_rate * 100.0);
            println!("   Failure: {:.1}%", results.simulated_outcomes.failure_rate * 100.0);
        }

        "coverage" => {
            println!("Analyzing coverage...");
            let analysis = eval::coverage::analyze_coverage(&conn)?;
            eval::coverage::print_coverage_analysis(&analysis);
        }

        "all" => {
            println!("Running full benchmark suite...\n");

            // Coverage analysis (always available)
            println!("1/3 Coverage Analysis...");
            let coverage = eval::coverage::analyze_coverage(&conn)?;

            // Monte Carlo (always available)
            println!("2/3 Monte Carlo (1000 simulations)...");
            let mc_config = eval::monte_carlo::MonteCarloConfig {
                num_simulations: 1000,
                ..Default::default()
            };
            let monte_carlo = eval::monte_carlo::run_simulation(&conn, &provenance, &mc_config)?;

            // Scenarios (if directory exists)
            println!("3/3 Scenario benchmarks...");
            let scenario_dir = data_dir.join("scenarios");
            let scenarios = eval::scenarios::load_all_scenarios(&scenario_dir).unwrap_or_default();
            let scenario_results = if !scenarios.is_empty() {
                Some(eval::scenarios::run_benchmark(&conn, &provenance, &scenarios)?)
            } else {
                println!("   (no scenarios found in {:?})", scenario_dir);
                None
            };

            // Build report
            let timestamp = chrono::Utc::now().to_rfc3339();
            let mut report = eval::EvalReport {
                timestamp,
                scenario_results,
                monte_carlo_results: Some(monte_carlo),
                coverage_analysis: Some(coverage),
                judge_results: None, // Requires API key
                summary: eval::EvalSummary {
                    overall_score: 0.0,
                    strengths: vec![],
                    weaknesses: vec![],
                    recommendations: vec![],
                },
            };

            report.generate_summary();
            eval::print_eval_report(&report);

            // Save JSON report
            let report_path = data_dir.join("benchmark_report.json");
            let json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&report_path, &json)?;
            println!("\n📄 Full report saved to: {:?}", report_path);
        }

        "synthetic" => {
            let count = args.first()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000);

            let output_path = args.get(1).map(|s| PathBuf::from(s));

            println!("🧪 Generating {} synthetic questions...\n", count);

            let config = eval::synthetic::GeneratorConfig::default();
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(42);

            let questions = eval::synthetic::generate_sample(&config, count, seed);

            // Print stats
            let mut domain_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for q in &questions {
                *domain_counts.entry(q.domain.clone()).or_insert(0) += 1;
            }

            println!("DOMAIN DISTRIBUTION:");
            let mut sorted: Vec<_> = domain_counts.iter().collect();
            sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (domain, count) in sorted {
                let pct = *count as f64 / questions.len() as f64 * 100.0;
                let bar = "█".repeat((pct / 2.0) as usize);
                println!("   {:15} {:5} ({:5.1}%) {}", domain, count, pct, bar);
            }

            println!("\nSAMPLE QUESTIONS:");
            for q in questions.iter().take(10) {
                println!("   [{}] {}", q.domain, q.question);
            }

            // Save if output path provided
            if let Some(path) = output_path {
                let json = serde_json::to_string_pretty(&questions)?;
                std::fs::write(&path, json)?;
                println!("\n📄 Saved {} questions to {:?}", questions.len(), path);
            }

            println!("\n✨ Use these with: 100minds --benchmark scenarios <path>");
        }

        "data-driven" => {
            // Data-driven evaluation: no hardcoded expectations, learn from heuristics
            let count = args.first()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500);

            println!("📊 Running data-driven evaluation on {} synthetic questions...\n", count);
            println!("This evaluation uses heuristics (not hardcoded expectations).");
            println!("It measures: principle diversity, thinker coverage, domain relevance.\n");

            let results = eval::data_driven::run_fast_evaluation(&conn, &provenance, count)?;
            eval::data_driven::print_data_driven_results(&results);

            // Save JSON results
            let report_path = data_dir.join("data_driven_report.json");
            let json = serde_json::to_string_pretty(&results)?;
            std::fs::write(&report_path, &json)?;
            println!("\n📄 Report saved to: {:?}", report_path);
        }

        "eval-synthetic" => {
            // Generate + evaluate in one step
            let count = args.first()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100);

            println!("🧪 Generating and evaluating {} synthetic questions...\n", count);

            let config = eval::synthetic::GeneratorConfig::default();
            let questions = eval::synthetic::generate_sample(&config, count, 42);

            let engine = CounselEngine::new(&conn, &provenance);

            let mut total_score = 0.0;
            let mut domain_scores: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

            for (i, q) in questions.iter().enumerate() {
                let request = CounselRequest {
                    question: q.question.clone(),
                    context: CounselContext {
                        domain: Some(q.domain.clone()),
                        ..Default::default()
                    },
                };

                if let Ok(response) = engine.counsel(&request) {
                    let principles_count = response.positions.iter()
                        .map(|p| p.principles_cited.len())
                        .sum::<usize>();
                    let thinkers_count = response.positions.len();

                    // Simple quality score: diversity + relevance proxy
                    let score = (thinkers_count.min(5) as f64 / 5.0) * 0.5
                        + (principles_count.min(10) as f64 / 10.0) * 0.5;

                    total_score += score;
                    domain_scores.entry(q.domain.clone()).or_default().push(score);

                    if i < 5 {
                        println!("[{}] Q: {}", i + 1, q.question);
                        println!("    Thinkers: {}, Principles: {}, Score: {:.2}",
                                 thinkers_count, principles_count, score);
                        println!();
                    }
                }

                if (i + 1) % 25 == 0 {
                    println!("   Evaluated {}/{}", i + 1, count);
                }
            }

            println!("\nRESULTS:");
            println!("   Overall avg score: {:.2}/1.00", total_score / count as f64);

            println!("\n   By domain:");
            for (domain, scores) in &domain_scores {
                let avg = scores.iter().sum::<f64>() / scores.len() as f64;
                println!("      {:15} {:.2} (n={})", domain, avg, scores.len());
            }
        }

        _ => {
            println!("Unknown benchmark command: {}", subcommand);
            println!("\nUsage: 100minds --benchmark <command>");
            println!("\nCommands:");
            println!("  scenarios [dir]     Run scenario benchmarks (hardcoded expectations)");
            println!("  monte-carlo [n]     Run n Monte Carlo simulations (default 1000)");
            println!("  coverage            Analyze thinker/principle coverage");
            println!("  synthetic [n] [out] Generate n synthetic questions");
            println!("  eval-synthetic [n]  Generate + evaluate synthetic questions");
            println!("  data-driven [n]     DATA-DRIVEN evaluation (no hardcoded expectations)");
            println!("  all                 Run full benchmark suite");
        }
    }

    Ok(())
}

/// Run analysis commands
fn run_analyze(subcommand: &str) -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    match subcommand {
        "thinkers" => {
            let analysis = eval::coverage::analyze_coverage(&conn)?;

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 👥 THINKER ANALYSIS                                         │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            // Sort by utilization
            let mut sorted: Vec<_> = analysis.thinker_utilization.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

            println!("UTILIZATION RANKING:");
            for (i, (name, rate)) in sorted.iter().enumerate() {
                let bar = "█".repeat((**rate * 20.0) as usize);
                println!("   {:3}. {:30} {:5.1}% {}", i + 1, name, *rate * 100.0, bar);
            }
            println!();

            // Recommendations
            if !analysis.recommended_removals.is_empty() {
                println!("CONSIDER REMOVING (low utilization):");
                for name in &analysis.recommended_removals {
                    println!("   • {}", name);
                }
            }
        }

        "principles" => {
            let analysis = eval::coverage::analyze_coverage(&conn)?;

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 📚 PRINCIPLE ANALYSIS                                       │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            // Orphans
            println!("ORPHAN PRINCIPLES ({} never selected):", analysis.orphan_principles.len());
            for p in &analysis.orphan_principles {
                println!("   • {}", p);
            }
            println!();

            // Redundancy
            if !analysis.principle_redundancy.is_empty() {
                println!("POTENTIALLY REDUNDANT PAIRS:");
                for (a, b, sim) in &analysis.principle_redundancy {
                    println!("   {:.0}% - \"{}\" ↔ \"{}\"", sim * 100.0, a, b);
                }
            }
        }

        "domains" => {
            let analysis = eval::coverage::analyze_coverage(&conn)?;

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 🏷️ DOMAIN ANALYSIS                                          │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            // Sort by coverage
            let mut sorted: Vec<_> = analysis.domain_coverage.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

            println!("DOMAIN COVERAGE:");
            for (domain, coverage) in &sorted {
                let bar = "█".repeat((**coverage * 20.0) as usize);
                println!("   {:25} {:5.1}% {}", domain, *coverage * 100.0, bar);
            }
            println!();

            // Gaps
            if !analysis.recommended_additions.is_empty() {
                println!("RECOMMENDED ADDITIONS:");
                for suggestion in &analysis.recommended_additions {
                    println!("   ➕ {} ({})", suggestion.name, suggestion.domain);
                    println!("      {}", suggestion.reason);
                }
            }
        }

        "coverage" | _ => {
            let analysis = eval::coverage::analyze_coverage(&conn)?;
            eval::coverage::print_coverage_analysis(&analysis);
        }
    }

    Ok(())
}

/// Run Thompson Sampling commands
fn run_thompson(subcommand: &str) -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    match subcommand {
        "stats" => {
            println!("Loading Thompson Sampling statistics...");

            let selector = eval::thompson::ThompsonSelector::from_db(&conn)?;
            let stats = selector.get_all_stats();

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ 🎰 THOMPSON SAMPLING STATISTICS                             │");
            println!("└─────────────────────────────────────────────────────────────┘\n");

            println!("TOP 20 PRINCIPLES BY MEAN:");
            for (i, stat) in stats.iter().take(20).enumerate() {
                let ci = format!("[{:.2}, {:.2}]", stat.ci_lower, stat.ci_upper);
                println!("   {:2}. {:35} mean: {:.2}  CI: {:15}  n: {:.0}",
                         i + 1, truncate_str(&stat.name, 35), stat.mean, ci, stat.total_observations);
            }
            println!();

            println!("BOTTOM 10 (needs more data or poor performance):");
            for stat in stats.iter().rev().take(10) {
                let ci = format!("[{:.2}, {:.2}]", stat.ci_lower, stat.ci_upper);
                let ci_width = stat.ci_upper - stat.ci_lower;
                let uncertainty = if ci_width > 0.5 { "⚠️ high uncertainty" } else { "" };
                println!("   {:35} mean: {:.2}  CI: {:15}  {}",
                         truncate_str(&stat.name, 35), stat.mean, ci, uncertainty);
            }
        }

        "init" => {
            println!("Initializing Thompson Sampling schema...");
            eval::thompson::init_thompson_schema(&conn)?;
            println!("Done. Thompson arms table created.");
        }

        "persist" => {
            println!("Persisting Thompson parameters to database...");
            let selector = eval::thompson::ThompsonSelector::from_db(&conn)?;
            selector.persist_to_db(&conn)?;
            println!("Done. learned_confidence updated from Thompson means.");
        }

        "remediate" => {
            println!("Analyzing poor performers for remediation...\n");

            // Find principles with low confidence
            let mut stmt = conn.prepare(
                "SELECT id, name, learned_confidence,
                        (SELECT COUNT(*) FROM framework_adjustments WHERE principle_id = principles.id) as adj_count
                 FROM principles
                 WHERE learned_confidence < 0.3
                 ORDER BY learned_confidence ASC"
            )?;

            let poor_performers: Vec<(String, String, f64, i64)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                .filter_map(|r| r.ok())
                .collect();

            if poor_performers.is_empty() {
                println!("✅ No poor performers found (all principles >= 0.3 confidence)");
                return Ok(());
            }

            println!("Found {} poor performers:\n", poor_performers.len());

            let mut to_reset = Vec::new();
            let mut to_archive = Vec::new();

            for (id, name, conf, adj_count) in &poor_performers {
                if *adj_count < 5 {
                    // Not enough data - give second chance
                    println!("   🔄 RESET: {} (conf: {:.2}, only {} samples)",
                             truncate_str(name, 40), conf, adj_count);
                    to_reset.push(id.clone());
                } else if *conf < 0.15 {
                    // Sufficient data, still failing - archive
                    println!("   📦 ARCHIVE: {} (conf: {:.2}, {} samples)",
                             truncate_str(name, 40), conf, adj_count);
                    to_archive.push(id.clone());
                } else {
                    // Low but recovering - leave alone
                    println!("   ⏳ WATCH: {} (conf: {:.2}, {} samples)",
                             truncate_str(name, 40), conf, adj_count);
                }
            }

            println!();

            // Apply remediation
            if !to_reset.is_empty() {
                println!("Resetting {} principles to baseline (0.5)...", to_reset.len());
                for id in &to_reset {
                    conn.execute(
                        "UPDATE principles SET learned_confidence = 0.5 WHERE id = ?1",
                        [id]
                    )?;
                }
                println!("✅ Reset complete");
            }

            if !to_archive.is_empty() {
                println!("\n⚠️  {} principles marked for archive (confidence < 0.15 with 5+ samples)", to_archive.len());
                println!("   Run with --thompson archive to move them to inactive status");
            }

            println!("\n📊 Summary:");
            println!("   Reset (second chance): {}", to_reset.len());
            println!("   Recommend archive: {}", to_archive.len());
            println!("   Watching (recovering): {}", poor_performers.len() - to_reset.len() - to_archive.len());
        }

        "explore" => {
            // Epsilon-greedy exploration: boost random poor performers
            println!("Running exploration round (boosting 10 random poor performers)...\n");

            conn.execute(
                "UPDATE principles
                 SET learned_confidence = 0.6
                 WHERE id IN (
                     SELECT id FROM principles
                     WHERE learned_confidence < 0.3
                     ORDER BY RANDOM()
                     LIMIT 10
                 )",
                []
            )?;

            println!("✅ Boosted 10 random poor performers to 0.6 confidence");
            println!("   These will now be selected more often to gather data.");
            println!("   Run --thompson persist after outcomes to update from real data.");
        }

        "cull" => {
            // Auto-cull hopeless principles
            println!("🗑️  AUTO-CULL: Removing hopeless principles...\n");

            // Find principles to cull: low confidence + sufficient data
            let culled: Vec<(String, String, f64, i64)> = conn.prepare(
                "SELECT id, name, learned_confidence,
                        (SELECT COUNT(*) FROM framework_adjustments WHERE principle_id = principles.id)
                 FROM principles
                 WHERE learned_confidence < 0.15
                 AND (SELECT COUNT(*) FROM framework_adjustments WHERE principle_id = principles.id) >= 5"
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .filter_map(|r| r.ok())
            .collect();

            if culled.is_empty() {
                println!("✅ No principles to cull (none below 0.15 with 5+ samples)");
                return Ok(());
            }

            println!("Culling {} principles:\n", culled.len());
            for (id, name, conf, samples) in &culled {
                println!("   ❌ {} (conf: {:.2}, {} samples)", name, conf, samples);

                // Archive by copying to archive table, then soft-delete (set confidence to -1)
                conn.execute(
                    "INSERT OR REPLACE INTO archived_principles
                     (id, thinker_id, name, description, domain_tags, application_rule,
                      anti_pattern, falsification, base_confidence, learned_confidence,
                      archived_at, cull_reason)
                     SELECT id, thinker_id, name, description, domain_tags, application_rule,
                            anti_pattern, falsification, base_confidence, learned_confidence,
                            datetime('now'), 'low_confidence'
                     FROM principles WHERE id = ?1",
                    [id]
                )?;
                // Soft-delete by setting confidence to -1 (excluded from searches)
                conn.execute(
                    "UPDATE principles SET learned_confidence = -1.0 WHERE id = ?1",
                    [id]
                )?;
            }

            // Rebuild FTS index
            conn.execute("INSERT INTO principles_fts(principles_fts) VALUES('rebuild')", [])?;

            println!("\n✅ Culled {} principles (archived, not deleted)", culled.len());
            println!("   To restore: SELECT * FROM archived_principles");
        }

        "discover" => {
            // Mine patterns from successful outcomes to discover new principle candidates
            println!("🔍 DISCOVERY: Mining patterns from successful outcomes...\n");

            // Get successful decisions with their context
            let successes: Vec<(String, String)> = conn.prepare(
                "SELECT question, context_json FROM decisions
                 WHERE outcome_success = 1
                 ORDER BY outcome_recorded_at DESC
                 LIMIT 500"
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default())))?
            .filter_map(|r| r.ok())
            .collect();

            println!("Analyzing {} successful decisions...\n", successes.len());

            // Extract common patterns/keywords
            let mut keyword_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let stopwords = ["should", "we", "the", "a", "an", "to", "for", "is", "it", "our", "use", "add", "do", "can", "be", "this", "that", "with", "from", "or", "and", "in", "on", "of", "how", "what", "when", "why"];

            for (question, _context) in &successes {
                for word in question.to_lowercase().split_whitespace() {
                    let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if clean.len() > 3 && !stopwords.contains(&clean) {
                        *keyword_counts.entry(clean.to_string()).or_insert(0) += 1;
                    }
                }
            }

            // Find patterns that appear often in successes
            let mut sorted: Vec<_> = keyword_counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            println!("TOP KEYWORDS IN SUCCESSFUL DECISIONS:");
            for (word, count) in sorted.iter().take(20) {
                let bar = "█".repeat(*count / 5);
                println!("   {:20} {:4} {}", word, count, bar);
            }

            // Get top principles from successes
            println!("\nTOP PRINCIPLES IN SUCCESSFUL OUTCOMES:");
            let top_principles: Vec<(String, i64)> = conn.prepare(
                "SELECT p.name, COUNT(*) as cnt
                 FROM framework_adjustments fa
                 JOIN principles p ON fa.principle_id = p.id
                 WHERE fa.success = 1
                 GROUP BY fa.principle_id
                 ORDER BY cnt DESC
                 LIMIT 15"
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

            for (name, count) in &top_principles {
                let bar = "█".repeat(*count as usize / 3);
                println!("   {:40} {:3} {}", truncate_str(name, 40), count, bar);
            }

            // Suggest new principles based on gaps
            println!("\n💡 SUGGESTED NEW PRINCIPLES (based on patterns):");
            let suggestions = vec![
                ("Pattern: 'microservices' frequent", "Consider adding: 'Start Monolith, Extract Later' from Sam Newman"),
                ("Pattern: 'scale/scaling' frequent", "Consider adding: 'Horizontal vs Vertical Scaling' heuristics"),
                ("Pattern: 'team/teams' frequent", "Consider adding: 'Inverse Conway Maneuver' from Team Topologies"),
            ];

            for (pattern, suggestion) in suggestions {
                if sorted.iter().any(|(w, c)| *c > 10 && pattern.to_lowercase().contains(w)) {
                    println!("   • {}", suggestion);
                }
            }

            println!("\n📊 Run --thompson yuzu to generate Yuzu-compatible actions");
        }

        "yuzu" => {
            // Generate Yuzu-compatible actions for the learning loop
            println!("🍋 YUZU INTEGRATION: Generating automated actions...\n");

            // Count what needs attention
            let cull_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM principles
                 WHERE learned_confidence < 0.15
                 AND (SELECT COUNT(*) FROM framework_adjustments WHERE principle_id = principles.id) >= 5",
                [],
                |row| row.get(0)
            )?;

            let explore_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM principles WHERE learned_confidence < 0.3",
                [],
                |row| row.get(0)
            )?;

            let total_outcomes: i64 = conn.query_row(
                "SELECT COUNT(*) FROM decisions WHERE outcome_success IS NOT NULL",
                [],
                |row| row.get(0)
            )?;

            // Output JSON for Yuzu to consume
            let actions = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status": {
                    "total_principles": 482,
                    "total_outcomes": total_outcomes,
                    "cull_candidates": cull_count,
                    "explore_candidates": explore_count
                },
                "actions": [
                    {
                        "type": "cull",
                        "count": cull_count,
                        "command": "100minds --thompson cull",
                        "priority": if cull_count > 20 { "high" } else { "low" }
                    },
                    {
                        "type": "explore",
                        "count": explore_count,
                        "command": "100minds --thompson explore",
                        "priority": if explore_count > 50 { "medium" } else { "low" }
                    },
                    {
                        "type": "persist",
                        "command": "100minds --thompson persist",
                        "priority": "always"
                    },
                    {
                        "type": "discover",
                        "command": "100minds --thompson discover",
                        "priority": if total_outcomes > 100 { "medium" } else { "low" }
                    }
                ],
                "recommended_schedule": {
                    "cull": "weekly",
                    "explore": "daily",
                    "persist": "after_each_batch",
                    "discover": "weekly"
                }
            });

            println!("{}", serde_json::to_string_pretty(&actions)?);

            println!("\n---");
            println!("Add to Yuzu daemon cron:");
            println!("  0 * * * *  100minds --thompson persist  # hourly");
            println!("  0 6 * * *  100minds --thompson explore  # daily 6am");
            println!("  0 0 * * 0  100minds --thompson cull     # weekly Sunday midnight");
            println!("  0 0 * * 0  100minds --thompson discover # weekly Sunday midnight");
        }

        "contextual" => {
            // Initialize contextual Thompson Sampling from existing data
            println!("🎯 CONTEXTUAL LEARNING: Initializing domain-specific Thompson arms...\n");

            // Create contextual_arms table if needed
            conn.execute(
                "CREATE TABLE IF NOT EXISTS contextual_arms (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    principle_id TEXT NOT NULL,
                    domain TEXT NOT NULL,
                    alpha REAL DEFAULT 1.0,
                    beta REAL DEFAULT 1.0,
                    sample_count INTEGER DEFAULT 0,
                    last_updated TEXT DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(principle_id, domain)
                )",
                [],
            )?;

            // Domains to create arms for
            let domains = [
                ("software-architecture", "architecture"),
                ("testing", "testing"),
                ("entrepreneurship", "entrepreneurship"),
                ("management-theory", "management"),
                ("ai-ml", "ai"),
                ("philosophy-ethics", "ethics"),
                ("systems-thinking", "systems"),
                ("software-practices", "practices"),
            ];

            let mut total_created = 0;

            for (domain_tag, domain_name) in &domains {
                // Find principles with this domain tag
                let principles: Vec<(String, String, f64)> = conn.prepare(
                    "SELECT id, name, learned_confidence FROM principles
                     WHERE domain_tags LIKE ?1 AND learned_confidence > 0"
                )?
                .query_map([format!("%{}%", domain_tag)], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

                let count = principles.len();
                if count > 0 {
                    println!("  📂 {} domain: {} principles", domain_name, count);

                    for (id, _name, conf) in &principles {
                        // Initialize with prior based on learned_confidence
                        let alpha = 1.0 + conf * 5.0;  // Higher confidence = more successes
                        let beta = 1.0 + (1.0 - conf) * 5.0;  // Lower confidence = more failures

                        conn.execute(
                            "INSERT OR IGNORE INTO contextual_arms
                             (principle_id, domain, alpha, beta, sample_count)
                             VALUES (?1, ?2, ?3, ?4, 10)",
                            rusqlite::params![id, domain_name, alpha, beta],
                        )?;
                        total_created += 1;
                    }
                }
            }

            // Also create arms for principles based on keyword matching in name/description
            let keyword_mappings = [
                ("Test", "testing"),
                ("TDD", "testing"),
                ("Build", "architecture"),
                ("Microservice", "architecture"),
                ("Scale", "systems"),
                ("Performance", "systems"),
                ("Team", "management"),
                ("Brooks", "management"),
                ("YAGNI", "practices"),
                ("Simplicity", "practices"),
            ];

            for (keyword, domain) in &keyword_mappings {
                let principles: Vec<(String, f64)> = conn.prepare(
                    "SELECT id, learned_confidence FROM principles
                     WHERE (name LIKE ?1 OR description LIKE ?1) AND learned_confidence > 0"
                )?
                .query_map([format!("%{}%", keyword)], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

                for (id, conf) in &principles {
                    let alpha = 1.0 + conf * 5.0;
                    let beta = 1.0 + (1.0 - conf) * 5.0;

                    conn.execute(
                        "INSERT OR IGNORE INTO contextual_arms
                         (principle_id, domain, alpha, beta, sample_count)
                         VALUES (?1, ?2, ?3, ?4, 10)",
                        rusqlite::params![id, domain, alpha, beta],
                    )?;
                }
            }

            let final_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM contextual_arms",
                [],
                |row| row.get(0),
            )?;

            println!("\n✅ Created {} contextual arms across all domains", final_count);
            println!("   Contextual learning will now adjust confidence per-domain.");
            println!("\n   To view: sqlite3 wisdom.db 'SELECT domain, COUNT(*) FROM contextual_arms GROUP BY domain'");
        }

        "decay" => {
            // Apply temporal decay to old adjustments (recent outcomes matter more)
            println!("⏳ TEMPORAL DECAY: Weighting recent outcomes more heavily...\n");

            // Get total adjustments
            let total_before: i64 = conn.query_row(
                "SELECT COUNT(*) FROM framework_adjustments",
                [],
                |row| row.get(0),
            )?;

            println!("   Total adjustments: {}", total_before);

            // Apply decay: reduce weight of old adjustments by recalculating learned_confidence
            // using exponentially weighted moving average with λ = 0.95 per day
            // This means 1-week old data has weight 0.95^7 ≈ 0.70
            // 1-month old data has weight 0.95^30 ≈ 0.21

            // Reset all principles to base confidence
            conn.execute(
                "UPDATE principles SET learned_confidence = base_confidence",
                [],
            )?;

            // Recalculate with time-weighted adjustments in Rust
            // Recent adjustments get full weight, older ones get decayed
            let adjustments: Vec<(String, f64, String)> = conn.prepare(
                "SELECT principle_id, adjustment, created_at
                 FROM framework_adjustments
                 WHERE created_at IS NOT NULL"
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|r| r.ok())
            .collect();

            // Calculate days since each adjustment and apply decay
            let now = chrono::Utc::now();
            let mut decayed_by_principle: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

            for (principle_id, adjustment, created_at) in &adjustments {
                // Parse date and calculate days ago
                let days_ago = if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S") {
                    let created = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
                    (now - created).num_days() as f64
                } else {
                    0.0  // Recent if unparseable
                };

                // Decay factor: 0.95^days (half-life ≈ 13 days)
                let decay = 0.95_f64.powf(days_ago);
                *decayed_by_principle.entry(principle_id.clone()).or_insert(0.0) += adjustment * decay;
            }

            let decayed_adjustments: Vec<(String, f64)> = decayed_by_principle.into_iter().collect();

            let mut updated = 0;
            for (principle_id, decayed_adj) in &decayed_adjustments {
                conn.execute(
                    "UPDATE principles
                     SET learned_confidence = MIN(1.0, MAX(0.0, base_confidence + ?2))
                     WHERE id = ?1",
                    rusqlite::params![principle_id, decayed_adj],
                )?;
                updated += 1;
            }

            // Also decay contextual arms
            conn.execute(
                "UPDATE contextual_arms
                 SET alpha = 1.0 + (alpha - 1.0) * 0.9,
                     beta = 1.0 + (beta - 1.0) * 0.9
                 WHERE sample_count > 50",
                [],
            )?;

            println!("   Updated {} principles with time-weighted confidence", updated);
            println!("   Decayed contextual arms with >50 samples");
            println!("\n✅ Temporal decay applied. Recent outcomes now weighted more heavily.");
        }

        _ => {
            println!("Unknown thompson command: {}", subcommand);
            println!("\nUsage: 100minds --thompson <command>");
            println!("\nCommands:");
            println!("  stats       Show Thompson Sampling statistics for all principles");
            println!("  init        Initialize Thompson Sampling database tables");
            println!("  persist     Update learned_confidence from Thompson means");
            println!("  remediate   Fix poor performers (reset or archive)");
            println!("  explore     Boost random poor performers for exploration");
            println!("  contextual  Initialize domain-specific contextual learning");
            println!("  cull        Archive principles with consistently poor performance");
            println!("  discover    Mine patterns from successful outcomes");
            println!("  yuzu        Generate Yuzu-compatible automation actions");
        }
    }

    Ok(())
}

// ============================================================================
// Semantic Embeddings Commands
// ============================================================================

/// Pre-compute embeddings for all principles
fn run_compute_embeddings() -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🧠 COMPUTING SEMANTIC EMBEDDINGS                            │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Initialize embedding schema (adds embedding column if needed)
    embeddings::init_embedding_schema(&conn)?;

    // Initialize semantic engine (downloads model if needed)
    let model_dir = embeddings::get_model_dir();
    println!("Model directory: {:?}", model_dir);

    let mut engine = embeddings::SemanticEngine::new(&model_dir)?;
    println!("Semantic engine initialized.\n");

    // Compute embeddings for all principles
    let count = engine.compute_all_embeddings(&conn)?;

    println!("\n✅ Computed embeddings for {} principles", count);
    println!("   These embeddings enable vocabulary-mismatch-proof search.");
    println!("   Example: 'rewrite legacy system' will now find 'Strangler Fig Pattern'");

    Ok(())
}

/// Run hybrid semantic + BM25 search
fn run_hybrid_search(query: &str) -> Result<()> {
    if query.is_empty() {
        println!("Usage: 100minds --hybrid-search <query>");
        return Ok(());
    }

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🔍 HYBRID SEMANTIC SEARCH                                   │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Query: {}\n", query);

    // Initialize semantic engine
    let model_dir = embeddings::get_model_dir();
    let mut engine = match embeddings::SemanticEngine::new(&model_dir) {
        Ok(e) => e,
        Err(e) => {
            println!("⚠️  Semantic engine not available: {}", e);
            println!("   Run --compute-embeddings first to download the model.");
            return Ok(());
        }
    };

    // Load pre-computed embeddings
    let loaded = engine.load_embeddings(&conn)?;
    if loaded == 0 {
        println!("⚠️  No embeddings found. Run --compute-embeddings first.");
        return Ok(());
    }
    println!("Loaded {} principle embeddings\n", loaded);

    // Run hybrid search
    let results = engine.hybrid_search(&conn, query, 10, 0.6)?;

    println!("TOP 10 MATCHES (60% semantic, 40% BM25):\n");
    for (i, m) in results.iter().enumerate() {
        // Get principle details
        let details: (String, String, String) = conn.query_row(
            "SELECT p.name, t.name, p.description FROM principles p
             JOIN thinkers t ON p.thinker_id = t.id
             WHERE p.id = ?1",
            [&m.principle_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap_or_else(|_| (m.principle_id.clone(), "Unknown".into(), "".into()));

        let (name, thinker, desc) = details;

        println!("{}. {} ({:.2})", i + 1, name, m.combined_score);
        println!("   Thinker: {}", thinker);
        println!("   Semantic: {:.2}  BM25: {:.2}", m.semantic_score, m.bm25_score);
        println!("   {}", truncate_str(&desc, 70));
        println!();
    }

    Ok(())
}

// ============================================================================
// Outcome Recording Commands (THE FLYWHEEL ACTIVATOR)
// ============================================================================

/// Record an outcome for a decision
fn run_outcome_cmd(args: &[String]) -> Result<()> {
    if args.is_empty() {
        println!("Usage: 100minds --outcome <decision-id> --success|--failed --principles \"id1,id2\" [--notes \"...\"] [--context '{{\"domain\":\"...\"}}']\n");
        println!("Examples:");
        println!("  100minds --outcome abc123 --success --principles \"yagni,kiss\"");
        println!("  100minds --outcome abc123 --failed --principles \"brooks-law\" --notes \"Added too many people\"");
        println!("  100minds --outcome bead-bd-123 --success --principles \"strangler-fig\" --context '{{\"domain\":\"architecture\"}}'");
        return Ok(());
    }

    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    // Parse arguments
    let decision_id = &args[0];
    let mut success = true;
    let mut principles: Vec<String> = vec![];
    let mut notes = String::new();
    let mut context_pattern: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--success" => success = true,
            "--failed" | "--failure" => success = false,
            "--principles" => {
                if i + 1 < args.len() {
                    i += 1;
                    principles = args[i].split(',').map(|s| s.trim().to_string()).collect();
                }
            }
            "--notes" => {
                if i + 1 < args.len() {
                    i += 1;
                    notes = args[i].clone();
                }
            }
            "--context" => {
                if i + 1 < args.len() {
                    i += 1;
                    context_pattern = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│ 🔄 RECORDING OUTCOME (FLYWHEEL ACTIVATION)                  │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    println!("Decision: {}", decision_id);
    println!("Outcome: {}", if success { "✅ SUCCESS" } else { "❌ FAILURE" });
    println!("Principles: {:?}", principles);
    if !notes.is_empty() {
        println!("Notes: {}", notes);
    }
    println!();

    // Record the outcome
    let result = outcome::record_outcome(
        &conn,
        decision_id,
        success,
        &principles,
        &notes,
        context_pattern.as_deref(),
    )?;

    // Show adjustments
    if result.principles_adjusted.is_empty() {
        println!("⚠️  No principles were adjusted (check principle IDs exist)");
    } else {
        println!("CONFIDENCE ADJUSTMENTS:");
        for adj in &result.principles_adjusted {
            let arrow = if adj.delta > 0.0 { "↑" } else { "↓" };
            let color_hint = if adj.delta > 0.0 { "📈" } else { "📉" };
            println!("   {} {} {}: {:.0}% → {:.0}% ({}{:.0}%)",
                color_hint,
                arrow,
                adj.principle_name,
                adj.old_confidence * 100.0,
                adj.new_confidence * 100.0,
                if adj.delta > 0.0 { "+" } else { "" },
                adj.delta * 100.0
            );
        }
        println!();
        println!("✅ Flywheel activated! {} principles updated.", result.principles_adjusted.len());
        println!("   Asymmetric learning: failures hurt more (-10%) than successes help (+5%)");
        println!("   This implements Taleb's 'skin in the game' - bad advice is penalized heavily.");
    }

    Ok(())
}

/// Show learning flywheel statistics
fn run_learning_stats() -> Result<()> {
    let data_dir = get_data_dir()?;
    let db_path = data_dir.join("wisdom.db");
    let conn = db::init_db(&db_path)?;

    let stats = outcome::get_learning_stats(&conn)?;
    outcome::print_learning_stats(&stats);

    // Additional context
    if stats.total_outcomes > 0 {
        println!("\nHOW THE FLYWHEEL WORKS:");
        println!("   1. Agent asks for counsel → 100minds provides principles");
        println!("   2. Agent executes decision → records outcome with --outcome");
        println!("   3. Principles that led to success gain confidence (+5%)");
        println!("   4. Principles that led to failure lose confidence (-10%)");
        println!("   5. Next counsel uses learned_confidence for better ranking");
        println!();
        println!("INTEGRATION:");
        println!("   Zesty workers can record outcomes automatically:");
        println!("   zesty outcome record --bead bd-123 --success --principles \"yagni\"");
    }

    Ok(())
}

