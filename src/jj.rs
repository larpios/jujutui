use anyhow::Result;
use std::process::{Command, Output};

#[derive(Debug, Clone, Default)]
pub struct Revision {
    pub is_working_copy: bool,
    pub change_id: String,
    pub commit_id: String,
    pub author: String,
    pub description: String,
    pub is_immutable: bool,
    pub is_empty: bool,
    pub has_conflict: bool,
    pub bookmarks: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StatusFile {
    pub path: String,
    pub status: String,
}

pub fn get_log() -> Result<Vec<Revision>> {
    // Use \x1e (record separator) to handle multiline descriptions
    let template = r#"if(current_working_copy, "@", " ") ++ "|" ++ change_id.short() ++ "|" ++ commit_id.short() ++ "|" ++ author.name() ++ "|" ++ if(immutable, "1", "0") ++ "|" ++ if(empty, "1", "0") ++ "|" ++ if(conflict, "1", "0") ++ "|" ++ bookmarks.map(|b| b.name()).join(",") ++ "|" ++ tags.map(|t| t.name()).join(",") ++ "|" ++ description ++ "\x1e""#;

    let out = Command::new("jj")
        .args(["log", "-r", "all()", "-T", template, "--no-graph"])
        .output()?;

    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "jj log failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut revisions = Vec::new();
    for record in stdout.split('\x1e') {
        if record.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.splitn(10, '|').collect();
        if parts.len() >= 10 {
            revisions.push(Revision {
                is_working_copy: parts[0].trim() == "@",
                change_id: parts[1].to_string(),
                commit_id: parts[2].to_string(),
                author: parts[3].to_string(),
                is_immutable: parts[4] == "1",
                is_empty: parts[5] == "1",
                has_conflict: parts[6] == "1",
                bookmarks: parts[7]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect(),
                tags: parts[8]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect(),
                description: parts[9].to_string(),
            });
        }
    }
    Ok(revisions)
}

pub fn get_diff(revision: &str) -> Result<String> {
    let out = Command::new("jj")
        .args(["diff", "-r", revision, "--color", "always"])
        .output()?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "jj diff failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn abandon(revision: &str, ignore_immutable: bool) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.args(["abandon", "-r", revision]);
    if ignore_immutable {
        cmd.arg("--ignore-immutable");
    }
    check(cmd.output()?, "abandon")
}

pub fn squash(
    revisions: &[String],
    target: Option<&str>,
    files: &[String],
    ignore_immutable: bool,
) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.arg("squash");
    for rev in revisions {
        cmd.args(["-r", rev]);
    }
    if let Some(t) = target {
        cmd.args(["--into", t]);
    }
    if ignore_immutable {
        cmd.arg("--ignore-immutable");
    }
    for f in files {
        cmd.arg(f);
    }
    check(cmd.output()?, "squash")
}

pub fn new_revision(parent: &str) -> Result<()> {
    check(Command::new("jj").args(["new", parent]).output()?, "new")
}

pub fn edit_revision(revision: &str) -> Result<()> {
    check(
        Command::new("jj").args(["edit", "-r", revision]).output()?,
        "edit",
    )
}

pub fn describe(revision: &str, message: &str, ignore_immutable: bool) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.args(["describe", "-r", revision, "-m", message]);
    if ignore_immutable {
        cmd.arg("--ignore-immutable");
    }
    check(cmd.output()?, "describe")
}

pub fn undo() -> Result<()> {
    check(Command::new("jj").args(["undo"]).output()?, "undo")
}

pub fn duplicate(revision: &str) -> Result<()> {
    check(
        Command::new("jj").args(["duplicate", revision]).output()?,
        "duplicate",
    )
}

pub fn rebase(source: &str, destination: &str, ignore_immutable: bool) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.args(["rebase", "-r", source, "-d", destination]);
    if ignore_immutable {
        cmd.arg("--ignore-immutable");
    }
    check(cmd.output()?, "rebase")
}

pub fn split_args(revision: &str) -> Vec<String> {
    vec!["split".to_string(), "-r".to_string(), revision.to_string()]
}

pub fn restore(files: &[String], revision: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.arg("restore");
    if let Some(r) = revision {
        cmd.args(["-r", r]);
    }
    for f in files {
        cmd.arg(f);
    }
    check(cmd.output()?, "restore")
}

pub fn git_fetch() -> Result<String> {
    run_command("git fetch")
}

pub fn git_push() -> Result<String> {
    run_command("git push")
}

pub fn absorb(ignore_immutable: bool) -> Result<String> {
    let mut cmd = Command::new("jj");
    cmd.arg("absorb");
    if ignore_immutable {
        cmd.arg("--ignore-immutable");
    }
    let out = cmd.output()?;
    let combined = format!(
        "{}{} ",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        Ok(combined)
    } else {
        Err(anyhow::anyhow!("absorb failed: {}", combined))
    }
}

pub fn run_interactive(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("jj");
    cmd.args(args);
    let mut child = cmd.spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("jj command failed with status: {}", status))
    }
}

pub fn run_command(cmd: &str) -> Result<String> {
    let args: Vec<&str> = cmd
        .strip_prefix("jj ")
        .unwrap_or(cmd)
        .split_whitespace()
        .collect();

    let out = Command::new("jj").args(args).output()?;
    let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str("-- STDERR --\n");
        combined.push_str(&stderr);
    }
    Ok(combined)
}

pub fn get_status_files(revision: &str) -> Result<Vec<StatusFile>> {
    let out = Command::new("jj")
        .args(["diff", "--summary", "-r", revision])
        .output()?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "jj diff --summary failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut files = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            files.push(StatusFile {
                status: parts[0].to_string(),
                path: parts[1..].join(" "),
            });
        }
    }
    Ok(files)
}

fn check(out: Output, op: &str) -> Result<()> {
    if out.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "jj {} failed: {}",
            op,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
