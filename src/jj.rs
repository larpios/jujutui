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

pub fn get_log() -> Result<Vec<Revision>> {
    // Description is last so splitn(8) captures | chars inside it
    let template = r#"if(current_working_copy, "@", " ") ++ "|" ++ change_id.short() ++ "|" ++ commit_id.short() ++ "|" ++ author.name() ++ "|" ++ if(immutable, "1", "0") ++ "|" ++ if(empty, "1", "0") ++ "|" ++ if(conflict, "1", "0") ++ "|" ++ description.first_line() ++ "\n""#;

    let out = Command::new("jj")
        .args(["log", "-r", "all()", "-T", template, "--no-graph"])
        .output()?;

    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "jj log failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let mut revisions = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.splitn(8, '|').collect();
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
    if revisions.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new("jj");
    cmd.arg("squash");
    for rev in revisions {
        cmd.args(["-r", rev]);
    }
    check(cmd.output()?, "squash")
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
