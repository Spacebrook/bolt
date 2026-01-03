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
    let root_dir = manifest_dir.join("..").join("c-quadtree");
    if root_dir.exists() {
        if should_update_repo() && root_dir.join(".git").exists() {
            git_pull(&root_dir)?;
        }
        return Ok(root_dir);
    }

    let vendor_dir = manifest_dir.join("third_party").join("c-quadtree");
    if vendor_dir.exists() {
        if should_update_repo() && vendor_dir.join(".git").exists() {
            git_pull(&vendor_dir)?;
        }
        return Ok(vendor_dir);
    }

    if let Some(parent) = vendor_dir.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create c-quadtree vendor dir: {err}"))?;
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
    cmd.arg("-C")
        .arg(repo_dir)
        .arg("pull")
        .arg("--ff-only");
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
            "-march=native",
            "-DNDEBUG",
            "-DHEADLESS",
            "-D_GNU_SOURCE",
            "-DALLOC_DEBUG",
            "-Wall",
            "-Wno-unused-function",
            "-Wno-address-of-packed-member",
        ])
        .arg("-lm");
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
        return Err(format!(
            "c-quadtree benchmark failed: {stderr}"
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_output(output: &str) -> Result<CQuadMetrics, String> {
    let mut collide_ms = None;
    let mut update_ms = None;
    let mut normalize_ms = None;
    let mut query_ms = None;

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
