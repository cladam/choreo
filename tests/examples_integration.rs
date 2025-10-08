// rust
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn run_all_examples() {
    // Allow overriding the binary path (useful locally or in CI)
    let choreo_bin = env::var("CHOREO_BINARY").unwrap_or_else(|_| {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target");
        p.push("debug");
        let exe_name = format!("choreo{}", std::env::consts::EXE_SUFFIX);
        p.push(exe_name);
        p.to_string_lossy().into_owned()
    });

    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let skip_env = env::var("SKIP_EXAMPLES").unwrap_or_default();
    let skip: Vec<String> = skip_env
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let entries = fs::read_dir(&examples_dir).expect("failed to read examples directory");

    let mut found = 0u32;
    for entry in entries {
        let entry = entry.expect("failed to read entry");
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "chor" {
                    let fname = path.file_name().unwrap().to_string_lossy().into_owned();
                    if skip.iter().any(|s| *s == fname) {
                        eprintln!("skipping example: {}", fname);
                        continue;
                    }
                    found += 1;
                    eprintln!("running example: {}", fname);

                    let output = Command::new(&choreo_bin)
                        .arg("run")
                        .arg("--file")
                        .arg(path.as_os_str())
                        .arg("--verbose")
                        .output()
                        .expect("failed to spawn choreo binary");

                    if output.status.success() {
                        eprintln!(
                            "--- stdout ---\n{}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                    } else {
                        // Print output to help debugging CI/local failures
                        eprintln!(
                            "--- stdout ---\n{}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                        eprintln!(
                            "--- stderr ---\n{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        panic!("example {} failed with status: {:?}", fname, output.status);
                    }
                }
            }
        }
    }

    assert!(found > 0, "no examples found in `examples/`");
}
