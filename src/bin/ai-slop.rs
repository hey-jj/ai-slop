//! Thin CLI: parse arguments, call the library, map exits. stdout carries
//! only the machine-readable JSON result. Diagnostics go to stderr.

use ai_slop::{
    analyze, engine, policy, skill, waiver, AnalysisError, Config, InputFormat, Profile,
    VerifyOutcome, WaiverAuthority,
};
use lexopt::prelude::*;
use std::io::{Read, Write};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_VIOLATION: u8 = 10;
const EXIT_INSTRUMENTATION: u8 = 30;
const EXIT_UNSUPPORTED: u8 = 40;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("ai-slop: {e}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

/// Write one line to stdout, treating a write failure — a closed pipe, a
/// closed descriptor — as quiet truncation exactly like the guarded report
/// path. A downstream `head` must never turn into a panic backtrace; the
/// intended exit code is still returned by the caller.
fn emit_line(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{text}");
}

/// As `emit_line` without the trailing newline (policy snapshot output).
fn emit_raw(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "{text}");
}

fn run() -> Result<u8, lexopt::Error> {
    let mut parser = lexopt::Parser::from_env();
    let first = parser.next()?;
    match first {
        Some(Value(cmd)) => {
            let cmd = cmd.string()?;
            match cmd.as_str() {
                "check" | "analyze" => cmd_check(parser),
                "verify" => cmd_verify(parser),
                "policy" => cmd_policy(parser),
                other => {
                    eprintln!("ai-slop: unknown subcommand {other}");
                    Ok(EXIT_USAGE)
                }
            }
        }
        Some(Long("version")) | Some(Short('V')) => {
            emit_line(&format!("ai-slop {}", env!("CARGO_PKG_VERSION")));
            Ok(EXIT_OK)
        }
        Some(Long("help")) | Some(Short('h')) => {
            emit_line(usage());
            Ok(EXIT_OK)
        }
        None => {
            eprintln!("{}", usage());
            Ok(EXIT_USAGE)
        }
        Some(arg) => Err(arg.unexpected()),
    }
}

fn usage() -> &'static str {
    "usage:\n  \
     ai-slop check   [--profile <P>] [--format <F>] [--suggest] [--waivers <FILE>]\n                  \
     [--config <FILE>] [--max-bytes <N>] [--output json] [PATH | -]\n  \
     ai-slop verify  --approval <FILE> [PATH | -]\n  \
     ai-slop policy  digest | show | snapshot [--out SKILL.md]\n  \
     ai-slop --version | -V\n  \
     ai-slop --help | -h\n\
     \n\
     `analyze` is an alias for `check`. Input larger than --max-bytes\n\
     (default 2 MiB) is rejected as unsupported.\n\
     \n\
     profiles (--profile, required, no default):\n  \
     public-bug-report, commit-message, changelog, release-notes, readme,\n  \
     api-docs, cargo-metadata, internal-doc\n\
     \n\
     formats (--format, defaults to the profile's declared format):\n  \
     markdown, text, commit, manifest\n\
     \n\
     exit codes:\n  \
     0   completed with no violation and no unresolved candidate\n  \
     2   usage error\n  \
     10  violation findings, or a failed verify\n  \
     20  unresolved candidate findings\n  \
     30  instrumentation error, fail closed\n  \
     40  unsupported input, fail closed"
}

fn read_input(path: Option<&str>) -> Result<Vec<u8>, String> {
    match path {
        None | Some("-") => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| format!("stdin read: {e}"))?;
            Ok(buf)
        }
        Some(p) => std::fs::read(p).map_err(|e| format!("{p}: {e}")),
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn error_json(state: &str, message: &str) -> String {
    format!(
        "{{\"schema_version\":\"{}\",\"tool_version\":\"{}\",\"result_state\":{},\"error\":{}}}",
        ai_slop::SCHEMA_VERSION,
        ai_slop::TOOL_VERSION,
        ai_slop::report::escape_json_string(state),
        ai_slop::report::escape_json_string(message)
    )
}

/// Record a positional PATH, rejecting a second one: silently analyzing only
/// the last path would let a globbing caller believe every file was checked.
fn set_path(path: &mut Option<String>, value: String) -> Result<(), u8> {
    if path.is_some() {
        eprintln!(
            "ai-slop: unexpected second path {value}; exactly one PATH (or -) is accepted per run"
        );
        return Err(EXIT_USAGE);
    }
    *path = Some(value);
    Ok(())
}

fn cmd_check(mut parser: lexopt::Parser) -> Result<u8, lexopt::Error> {
    let mut profile: Option<String> = None;
    let mut format: Option<String> = None;
    let mut suggest = false;
    let mut waivers_path: Option<String> = None;
    let mut config_path: Option<String> = None;
    let mut max_bytes: Option<usize> = None;
    let mut output: Option<String> = None;
    let mut path: Option<String> = None;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("profile") => profile = Some(parser.value()?.string()?),
            Long("format") => format = Some(parser.value()?.string()?),
            Long("suggest") => suggest = true,
            Long("waivers") => waivers_path = Some(parser.value()?.string()?),
            Long("config") => config_path = Some(parser.value()?.string()?),
            Long("max-bytes") => {
                max_bytes = Some(parser.value()?.parse()?);
            }
            Long("output") => output = Some(parser.value()?.string()?),
            Long("help") | Short('h') => {
                emit_line(usage());
                return Ok(EXIT_OK);
            }
            Value(v) => {
                if let Err(code) = set_path(&mut path, v.string()?) {
                    return Ok(code);
                }
            }
            arg => return Err(arg.unexpected()),
        }
    }

    if let Some(o) = &output {
        if o != "json" {
            eprintln!("ai-slop: --output supports only json");
            return Ok(EXIT_USAGE);
        }
    }
    let Some(profile_name) = profile else {
        eprintln!("ai-slop: --profile is required (no default, no detection)");
        return Ok(EXIT_USAGE);
    };
    let Some(profile) = Profile::from_str(&profile_name) else {
        eprintln!("ai-slop: unknown profile {profile_name}");
        return Ok(EXIT_USAGE);
    };

    let mut config = Config::new(profile);
    config.suggest = suggest;
    config.now_unix = Some(now_unix());
    if let Some(n) = max_bytes {
        config.limits.max_bytes = n;
    }
    if let Some(f) = format {
        let Some(f) = InputFormat::from_str(&f) else {
            eprintln!("ai-slop: unknown format {f}");
            return Ok(EXIT_USAGE);
        };
        if !profile.supported_formats().contains(&f) {
            eprintln!(
                "ai-slop: format {} is outside the {} profile's supported set",
                f.as_str(),
                profile.as_str()
            );
            return Ok(EXIT_USAGE);
        }
        config.input_format = f;
    }
    if let Some(p) = &config_path {
        match load_deployment(p) {
            Ok(d) => config.deployment = d,
            Err(e) => {
                eprintln!("ai-slop: config {p}: {e}");
                return Ok(EXIT_USAGE);
            }
        }
    }
    if let Some(p) = &waivers_path {
        match load_waivers(p) {
            Ok(w) => config.waivers = w,
            Err(e) => {
                eprintln!("ai-slop: waivers {p}: {e}");
                return Ok(EXIT_USAGE);
            }
        }
    }

    let input = match read_input(path.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ai-slop: {e}");
            return Ok(EXIT_USAGE);
        }
    };

    match analyze(&input, &config) {
        Ok(report) => {
            for note in &report.coverage.notes {
                eprintln!("ai-slop: note: {note}");
            }
            // Serialize straight into the locked stdout: no whole-report
            // String, and a closed pipe is quiet truncation, not a panic.
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            match serde_json::to_writer(&mut out, &report) {
                Ok(()) => {
                    let _ = out.write_all(b"\n");
                }
                Err(e) if e.is_io() => {}
                Err(e) => {
                    let _ = writeln!(
                        out,
                        "{}",
                        error_json("instrumentation_error", &e.to_string())
                    );
                }
            }
            let _ = out.flush();
            Ok(report.exit_code() as u8)
        }
        Err(AnalysisError::Instrumentation(m)) => {
            eprintln!("ai-slop: instrumentation_error: {m}");
            emit_line(&error_json("instrumentation_error", &m));
            Ok(EXIT_INSTRUMENTATION)
        }
        Err(AnalysisError::UnsupportedInput(m)) => {
            eprintln!("ai-slop: unsupported_input: {m}");
            emit_line(&error_json("unsupported_input", &m));
            Ok(EXIT_UNSUPPORTED)
        }
        Err(AnalysisError::Usage(m)) => {
            eprintln!("ai-slop: {m}");
            Ok(EXIT_USAGE)
        }
    }
}

/// A present-but-wrong-type value is a usage error, never a silent skip: a
/// mistyped `expected_version = 0.1` must not quietly disable the checks
/// that key on it. Absent keys keep their fail-closed defaults.
fn want_str(t: &toml::value::Table, key: &str) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => Ok(Some(
            v.as_str()
                .ok_or_else(|| format!("{key} must be a string"))?
                .to_string(),
        )),
    }
}

fn want_str_array(t: &toml::value::Table, key: &str) -> Result<Option<Vec<String>>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => {
            let arr = v
                .as_array()
                .ok_or_else(|| format!("{key} must be an array of strings"))?;
            let mut out = Vec::new();
            for item in arr {
                out.push(
                    item.as_str()
                        .ok_or_else(|| format!("{key} entries must be strings"))?
                        .to_string(),
                );
            }
            Ok(Some(out))
        }
    }
}

fn load_deployment(path: &str) -> Result<ai_slop::Deployment, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
    let t = value.as_table().ok_or("config root must be a table")?;
    let mut d = ai_slop::Deployment::default();
    if let Some(v) = want_str(t, "waiver_authority")? {
        d.waiver_authority = Some(match v.as_str() {
            "human" => WaiverAuthority::Human,
            "orchestrator-agent" => WaiverAuthority::OrchestratorAgent,
            other => return Err(format!("unknown waiver_authority {other}")),
        });
    }
    if let Some(list) = want_str_array(t, "demote")? {
        d.demote = list;
    }
    d.expected_version = want_str(t, "expected_version")?;
    d.expected_license_wording = want_str(t, "expected_license_wording")?;
    d.expected_release_body = want_str(t, "expected_release_body")?;
    d.scrub_overrides = want_str_array(t, "scrub_overrides")?;
    if let Some(list) = want_str_array(t, "exempt_comment_markers")? {
        d.exempt_comment_markers = list;
    }
    Ok(d)
}

fn load_waivers(path: &str) -> Result<Vec<waiver::Waiver>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum WaiverFile {
        List(Vec<waiver::Waiver>),
        Wrapped { waivers: Vec<waiver::Waiver> },
    }
    let parsed: WaiverFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(match parsed {
        WaiverFile::List(w) => w,
        WaiverFile::Wrapped { waivers } => waivers,
    })
}

fn cmd_verify(mut parser: lexopt::Parser) -> Result<u8, lexopt::Error> {
    let mut approval_path: Option<String> = None;
    let mut path: Option<String> = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Long("approval") => approval_path = Some(parser.value()?.string()?),
            Long("help") | Short('h') => {
                emit_line(usage());
                return Ok(EXIT_OK);
            }
            Value(v) => {
                if let Err(code) = set_path(&mut path, v.string()?) {
                    return Ok(code);
                }
            }
            arg => return Err(arg.unexpected()),
        }
    }
    let Some(approval_path) = approval_path else {
        eprintln!("ai-slop: verify requires --approval <FILE>");
        return Ok(EXIT_USAGE);
    };
    let approval_text = match std::fs::read_to_string(&approval_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ai-slop: {approval_path}: {e}");
            return Ok(EXIT_USAGE);
        }
    };
    let approval: waiver::Approval = match serde_json::from_str(&approval_text) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ai-slop: approval parse: {e}");
            return Ok(EXIT_USAGE);
        }
    };
    let input = match read_input(path.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ai-slop: {e}");
            return Ok(EXIT_USAGE);
        }
    };
    match ai_slop::verify(&input, &approval, now_unix()) {
        VerifyOutcome::Verified => {
            emit_line("{\"verified\":true}");
            Ok(EXIT_OK)
        }
        VerifyOutcome::Mismatch(problems) => {
            for p in &problems {
                eprintln!("ai-slop: verify: {p}");
            }
            emit_line("{\"verified\":false}");
            Ok(EXIT_VIOLATION)
        }
    }
}

fn cmd_policy(mut parser: lexopt::Parser) -> Result<u8, lexopt::Error> {
    let mut sub: Option<String> = None;
    let mut out: Option<String> = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Long("out") => out = Some(parser.value()?.string()?),
            Long("help") | Short('h') => {
                emit_line(usage());
                return Ok(EXIT_OK);
            }
            Value(v) if sub.is_none() => sub = Some(v.string()?),
            arg => return Err(arg.unexpected()),
        }
    }
    match sub.as_deref() {
        Some("digest") => {
            emit_line(&policy::compute_digest());
            Ok(EXIT_OK)
        }
        Some("show") => match engine::compiled() {
            Ok(cp) => {
                #[derive(serde::Serialize)]
                struct RuleShow<'a> {
                    id: &'a str,
                    name: &'a str,
                    family: &'a str,
                    tier: &'a str,
                    lifecycle: &'a str,
                }
                let rules: Vec<RuleShow> = cp
                    .pkg
                    .rules
                    .iter()
                    .map(|r| RuleShow {
                        id: &r.id,
                        name: &r.name,
                        family: &r.family,
                        tier: r.tier.as_str(),
                        lifecycle: r.lifecycle.as_str(),
                    })
                    .collect();
                emit_line(
                    &serde_json::json!({
                        "version": cp.pkg.version,
                        "digest": cp.pkg.digest,
                        "rules": rules,
                    })
                    .to_string(),
                );
                Ok(EXIT_OK)
            }
            Err(e) => {
                eprintln!("ai-slop: {e}");
                Ok(EXIT_INSTRUMENTATION)
            }
        },
        Some("snapshot") => match engine::compiled() {
            Ok(cp) => {
                let snapshot = skill::generate(&cp.pkg);
                match out {
                    Some(p) => {
                        if let Err(e) = std::fs::write(&p, snapshot) {
                            eprintln!("ai-slop: {p}: {e}");
                            return Ok(EXIT_USAGE);
                        }
                        Ok(EXIT_OK)
                    }
                    None => {
                        emit_raw(&snapshot);
                        Ok(EXIT_OK)
                    }
                }
            }
            Err(e) => {
                eprintln!("ai-slop: {e}");
                Ok(EXIT_INSTRUMENTATION)
            }
        },
        _ => {
            eprintln!("ai-slop: policy expects digest, show, or snapshot");
            Ok(EXIT_USAGE)
        }
    }
}
