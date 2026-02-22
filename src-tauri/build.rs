use std::path::PathBuf;

fn main() {
    let triple = std::env::var("TARGET").unwrap_or_else(|_| "aarch64-apple-darwin".to_string());
    let profile = if std::env::var("PROFILE").as_deref() == Ok("release") {
        "release"
    } else {
        "debug"
    };

    let sidecar = PathBuf::from(format!("binaries/mcp-hub-bridge-{}", triple));
    std::fs::create_dir_all("binaries").ok();

    // Try to copy a previously-built bridge binary into the sidecar slot.
    // On the very first build the binary won't exist yet, so we create a
    // small placeholder to satisfy Tauri's resource-path check.  On every
    // subsequent build (including `tauri build`) the real binary from the
    // previous compilation is picked up automatically.
    let candidate = PathBuf::from(format!("target/{}/mcp-hub-bridge", profile));
    if candidate.exists() {
        std::fs::copy(&candidate, &sidecar).ok();
    } else if !sidecar.exists() {
        std::fs::write(
            &sidecar,
            "#!/bin/sh\necho 'mcp-hub-bridge: run cargo build first' >&2\nexit 1\n",
        )
        .ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o755)).ok();
        }
    }

    tauri_build::build()
}
