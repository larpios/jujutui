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
}

#[derive(Debug, Clone)]
pub struct StatusFile {
    pub path: String,
    pub status: String,
}

pub fn get_log() -> Result<Vec<Revision>> {
    // Use \x1e (record separator) to handle multiline descriptions
    let template = r#"if(current_working_copy, "@", " ") ++ "|" ++ change_id.short() ++ "|" ++ commit_id.short() ++ "|" ++ author.name() ++ "|" ++ if(immutable, "1", "0") ++ "|" ++ if(empty, "1", "0") ++ "|" ++ if(conflict, "1", "0") ++ "|" ++ description ++ "\x1e""#;

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
        if record.trim().is_empty() { continue; }
        let parts: Vec<&str> = record.splitn(8, '|').collect();
        if parts.len() >= 8 {
            revisions.push(Revision {
                is_working_copy: parts[0].trim() == "@",
                change_id: parts[1].to_string(),
                commit_id: parts[2].to_string(),
                author: parts[3].to_string(),
                is_immutable: parts[4] == "1",
                is_empty: parts[5] == "1",
                has_conflict: parts[6] == "1",
                description: parts[7].to_string(),
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

pub fn abandon(revision: &str) -> Result<()> {
    check(Command::new("jj").args(["abandon", "-r", revision]).output()?, "abandon")
}

pub fn squash(revisions: &[String]) -> Result<()> {
    for rev in revisions {
        check(Command::new("jj").args(["squash", "-r", rev]).output()?, "squash")?;
    }
    Ok(())
}

pub fn new_revision(parent: &str) -> Result<()> {
    check(Command::new("jj").args(["new", parent]).output()?, "new")
}

pub fn edit_revision(revision: &str) -> Result<()> {
    check(Command::new("jj").args(["edit", "-r", revision]).output()?, "edit")
}

pub fn describe(revision: &str, message: &str) -> Result<()> {
    check(
        Command::new("jj")
            .args(["describe", "-r", revision, "-m", message])
            .output()?,
        "describe",
    )
}

pub fn undo() -> Result<()> {
    check(Command::new("jj").args(["undo"]).output()?, "undo")
}

pub fn duplicate(revision: &str) -> Result<()> {
    check(Command::new("jj").args(["duplicate", revision]).output()?, "duplicate")
}

pub fn rebase(source: &str, destination: &str) -> Result<()> {
    check(
        Command::new("jj")
            .args(["rebase", "-r", source, "-d", destination])
            .output()?,
        "rebase",
    )
}

pub fn run_command(cmd: &str) -> Result<String> {
    let args: Vec<&str> = if cmd.starts_with("jj ") {
        cmd[3..].split_whitespace().collect()
    } else {
        cmd.split_whitespace().collect()
    };
    
    let out = Command::new("jj").args(args).output()?;
    let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        combined.push_str("\n--- ERRORS ---\n");
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
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
