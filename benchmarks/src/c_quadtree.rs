use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_REPO: &str = "https://github.com/supahero1/c-quadtree";

pub struct CQuadMetrics {
    pub collide_ms: f64,
    pub update_ms: f64,
    pub normalize_ms: f64,
    pub query_ms: f64,
    #[allow(dead_code)]
    pub query_nodes_per: Option<f64>,
    #[allow(dead_code)]
    pub query_entities_per: Option<f64>,
    pub node_count: Option<u32>,
    pub node_entities_count: Option<u32>,
    pub entity_count: Option<u32>,
}

pub fn run() -> Result<CQuadMetrics, String> {
    let repo_dir = ensure_repo()?;
    ensure_submodules(&repo_dir)?;
    let binary = build_headless(&repo_dir)?;
    let output = run_benchmark(&repo_dir, &binary)?;
    parse_output(&output)
}

fn ensure_repo() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("BOLT_C_QUADTREE_DIR") {
        return Ok(PathBuf::from(path));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor_dir = manifest_dir
        .join("third_party")
        .join("c-quadtree")
        .join("c-quadtree");
    if vendor_dir.exists() {
        if should_update_repo() && vendor_dir.join(".git").exists() {
            git_pull(&vendor_dir)?;
        }
        return Ok(vendor_dir);
    }

    if let Some(parent) = vendor_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create c-quadtree vendor dir: {err}"))?;
    }

    let repo_url = env::var("BOLT_C_QUADTREE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());
    let mut cmd = Command::new("git");
    cmd.arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(repo_url)
        .arg(&vendor_dir);
    run_command(cmd, "git clone c-quadtree")?;

    Ok(vendor_dir)
}

fn should_update_repo() -> bool {
    env::var("BOLT_C_QUADTREE_UPDATE")
        .ok()
        .map(|value| value != "0")
        .unwrap_or(false)
}

fn git_pull(repo_dir: &Path) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_dir).arg("pull").arg("--ff-only");
    run_command(cmd, "git pull c-quadtree")
}

fn ensure_submodules(repo_dir: &Path) -> Result<(), String> {
    if !repo_dir.join("alloc").exists() && repo_dir.join(".gitmodules").exists() {
        if repo_dir.join(".git").exists() {
            let mut cmd = Command::new("git");
            cmd.arg("-C")
                .arg(repo_dir)
                .arg("submodule")
                .arg("update")
                .arg("--init")
                .arg("--recursive");
            run_command(cmd, "git submodule update c-quadtree")?;
        }
    }

    if !repo_dir.join("alloc").exists() {
        return Err("c-quadtree alloc submodule missing".to_string());
    }

    Ok(())
}

fn build_headless(repo_dir: &Path) -> Result<PathBuf, String> {
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let bin_name = format!("test_headless{}", env::consts::EXE_SUFFIX);
    let bin_path = repo_dir.join(&bin_name);

    let mut cmd = Command::new(cc);
    cmd.current_dir(repo_dir)
        .arg("test.c")
        .arg("-o")
        .arg(&bin_name)
        .args([
            "-Ofast",
            "-std=gnu2x",
            "-march=native",
            "-DNDEBUG",
            "-DHEADLESS",
            "-DHEADLESS_TICKS=1000",
            "-D_GNU_SOURCE",
            "-DALLOC_DEBUG",
            "-Wall",
            "-Wno-unused-function",
            "-Wno-address-of-packed-member",
        ])
        .args(["-include", "stdbool.h"])
        .arg("-lm");
    if env::var("BOLT_C_QUADTREE_QUERY_STATS").ok().as_deref() == Some("1") {
        cmd.arg("-DQUADTREE_QUERY_STATS=1");
    }
    run_command(cmd, "build c-quadtree headless benchmark")?;

    Ok(bin_path)
}

fn run_benchmark(repo_dir: &Path, bin_path: &Path) -> Result<String, String> {
    let output = Command::new(bin_path)
        .current_dir(repo_dir)
        .output()
        .map_err(|err| format!("run c-quadtree benchmark: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("c-quadtree benchmark failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_output(output: &str) -> Result<CQuadMetrics, String> {
    let mut collide_ms = None;
    let mut update_ms = None;
    let mut normalize_ms = None;
    let mut query_ms = None;
    let mut query_nodes_per = None;
    let mut query_entities_per = None;
    let mut node_count = None;
    let mut node_entities_count = None;
    let mut entity_count = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(value) = parse_ms(trimmed, "Collide") {
            collide_ms = Some(value);
        } else if let Some(value) = parse_ms(trimmed, "Update") {
            update_ms = Some(value);
        } else if let Some(value) = parse_ms(trimmed, "Normalize") {
            normalize_ms = Some(value);
        } else if let Some(value) = parse_ms(trimmed, "1k Queries") {
            query_ms = Some(value);
        } else if let Some(rest) = trimmed.strip_prefix("Query stats: nodes/query ") {
            if let Some((nodes_str, entities_str)) = rest.split_once(", entities/query ") {
                if let Ok(nodes) = nodes_str.trim().parse::<f64>() {
                    query_nodes_per = Some(nodes);
                }
                if let Ok(entities) = entities_str.trim().parse::<f64>() {
                    query_entities_per = Some(entities);
                }
            }
        } else if let Some(value) = parse_u32(trimmed, "Nodes") {
            node_count = Some(value);
        } else if let Some(value) = parse_u32(trimmed, "Node entities") {
            node_entities_count = Some(value);
        } else if let Some(value) = parse_u32(trimmed, "Entities") {
            entity_count = Some(value);
        }
    }

    let missing = [
        ("Collide", collide_ms),
        ("Update", update_ms),
        ("Normalize", normalize_ms),
        ("1k Queries", query_ms),
    ]
    .iter()
    .filter_map(|(label, value)| value.is_none().then_some(*label))
    .collect::<Vec<_>>();

    if !missing.is_empty() {
        let tail = output
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "missing c-quadtree output fields: {} (tail: {tail})",
            missing.join(", ")
        ));
    }

    Ok(CQuadMetrics {
        collide_ms: collide_ms.unwrap(),
        update_ms: update_ms.unwrap(),
        normalize_ms: normalize_ms.unwrap(),
        query_ms: query_ms.unwrap(),
        query_nodes_per,
        query_entities_per,
        node_count,
        node_entities_count,
        entity_count,
    })
}

fn parse_ms(line: &str, label: &str) -> Option<f64> {
    let prefix = format!("{label}: ");
    let rest = line.strip_prefix(&prefix)?;
    let value = rest.trim_end_matches("ms").trim();
    value.parse::<f64>().ok()
}

fn run_command(mut cmd: Command, action: &str) -> Result<(), String> {
    let output = cmd.output().map_err(|err| format!("{action}: {err}"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{action} failed: {stdout}{stderr}"));
    }
    Ok(())
}

fn parse_u32(line: &str, label: &str) -> Option<u32> {
    let prefix = format!("{label}: ");
    let rest = line.strip_prefix(&prefix)?;
    rest.trim().parse::<u32>().ok()
}
