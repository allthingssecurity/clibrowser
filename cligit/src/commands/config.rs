use anyhow::Result;
use crate::cli::ConfigArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: ConfigArgs, ctx: Option<&GitContext>, out: &OutputConfig) -> Result<i32> {
    if args.list {
        // List all config entries via git CLI (git2 config iteration is cumbersome)
        let workdir = ctx.map(|c| c.workdir.clone()).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let output = std::process::Command::new("git")
            .args(["config", "--list"])
            .current_dir(&workdir)
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<ConfigResult> = text.lines().filter_map(|line| {
            let (k, v) = line.split_once('=')?;
            Some(ConfigResult { key: k.to_string(), value: Some(v.to_string()), scope: None })
        }).collect();
        let result = ConfigListResult { entries };
        if out.json { out.print_json(&result); }
        else { for e in &result.entries { out.print_human(&format!("{}={}", e.key, e.value.as_deref().unwrap_or(""))); } }
        return Ok(0);
    }

    let key = match args.key {
        Some(ref k) => k,
        None => { anyhow::bail!("Config key is required (or use --list)"); }
    };

    if let Some(ref val) = args.value {
        // Set mode
        let workdir = ctx.map(|c| c.workdir.clone()).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let mut cmd = vec!["config"];
        let scope_flag;
        if let Some(ref scope) = args.scope {
            scope_flag = format!("--{}", scope);
            cmd.push(&scope_flag);
        }
        cmd.push(key);
        cmd.push(val);
        std::process::Command::new("git")
            .args(&cmd)
            .current_dir(&workdir)
            .output()?;
        let result = WriteResult { message: format!("Set {} = {}", key, val) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    } else {
        // Get mode
        if let Some(ctx) = ctx {
            let config = ctx.repo.config()?;
            let value = config.get_string(key).ok();
            let result = ConfigResult { key: key.clone(), value, scope: args.scope };
            if out.json { out.print_json(&result); }
            else { out.print_human(&format!("{}", result.value.as_deref().unwrap_or("(not set)"))); }
        } else {
            anyhow::bail!("Not in a git repository");
        }
    }
    Ok(0)
}
