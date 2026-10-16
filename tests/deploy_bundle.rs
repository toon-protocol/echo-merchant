//! The deploy bundle's connector configuration, rendered the way `render.sh` renders it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const CONNECTOR_IMAGE: &str = "ghcr.io/toon-protocol/connector:rust-2026.09.29.1";

fn deploy_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("deploy")
}

/// Run `render.sh` in a scratch copy of the bundle and parse the `connector.toml` it writes.
/// Each call gets its own scratch directory, because the tests run in parallel.
fn render(solana: bool) -> toml::Table {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "echo-merchant-render-{}-{}",
        std::process::id(),
        CALLS.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("nginx")).unwrap();
    for file in [
        "render.sh",
        "connector.toml.template",
        "nginx/node.conf.template",
    ] {
        std::fs::copy(deploy_dir().join(file), scratch.join(file)).unwrap();
    }
    std::fs::write(
        scratch.join(".env"),
        format!(
            "DOMAIN=example.test\nOPERATOR_BEARER_TOKEN=token\nOPERATOR_WRITE_KEY=key\nSETTLEMENT_SOLANA={}\n",
            if solana { "on" } else { "off" }
        ),
    )
    .unwrap();
    let out = Command::new("bash")
        .arg("render.sh")
        .current_dir(&scratch)
        .output()
        .expect("run render.sh");
    assert!(
        out.status.success(),
        "render.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rendered = std::fs::read_to_string(scratch.join("connector.toml")).unwrap();
    let _ = std::fs::remove_dir_all(&scratch);
    rendered.parse().expect("rendered connector.toml is TOML")
}

#[test]
fn serves_g_toon_echo_at_the_echo_merchant_priced_one_plus_one_per_kib() {
    let config = render(false);
    let routes = config["routes"].as_array().expect("routes");
    assert_eq!(routes.len(), 1, "one route only");
    let route = &routes[0];
    assert_eq!(route["prefix"].as_str(), Some("g.toon.echo"));
    assert_eq!(
        route["handler_url"].as_str(),
        Some("http://echo-merchant:3000")
    );
    let price = route["price"]
        .as_table()
        .expect("price is a { base, per_kib } table");
    assert_eq!(price["base"].as_integer(), Some(1));
    assert_eq!(price["per_kib"].as_integer(), Some(1));
    assert_eq!(price.len(), 2);
    assert_eq!(config["node"]["addresses"][0].as_str(), Some("g.toon.echo"));
}

#[test]
fn accepts_base_sepolia_and_with_the_switch_on_solana_devnet() {
    let evm_only = render(false);
    assert!(evm_only["settlement"]
        .as_table()
        .unwrap()
        .contains_key("evm"));
    assert!(!evm_only["settlement"]
        .as_table()
        .unwrap()
        .contains_key("solana"));
    let both = render(true);
    assert!(both["settlement"]["evm"]["rpc_url"]
        .as_str()
        .unwrap()
        .contains("base-sepolia"));
    assert!(both["settlement"]["solana"]["rpc_url"]
        .as_str()
        .unwrap()
        .contains("devnet"));
}

#[test]
fn compose_pins_the_connector_and_runs_the_handler_the_route_names() {
    let compose = std::fs::read_to_string(deploy_dir().join("docker-compose.yml")).unwrap();
    let images: Vec<&str> = compose
        .lines()
        .filter_map(|l| l.trim().strip_prefix("image: "))
        .collect();
    assert!(
        images.contains(&CONNECTOR_IMAGE),
        "connector pin moved: {images:?}"
    );
    assert!(images.contains(&"ghcr.io/toon-protocol/echo-merchant:release"));
    assert!(
        compose.contains("\n  echo-merchant:\n"),
        "handler_url host is the compose service"
    );
    for image in images.iter().filter(|i| i.contains("connector")) {
        assert!(
            !image.ends_with(":latest") && !image.contains("release"),
            "{image}"
        );
    }
}

/// `docker compose config --format json` over the given files, with the real compose
/// merge semantics (profiles drop a service outright). `None` when docker is not
/// installed, so a machine without it skips rather than fails.
fn compose_config(files: &[&str]) -> Option<serde_json::Value> {
    let mut cmd = Command::new("docker");
    cmd.arg("compose").current_dir(deploy_dir());
    for f in files {
        cmd.args(["-f", f]);
    }
    let out = match cmd.args(["config", "--format", "json"]).output() {
        Ok(out) => out,
        Err(_) => {
            eprintln!("docker not installed; skipping the compose-config check");
            return None;
        }
    };
    assert!(
        out.status.success(),
        "docker compose config failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(serde_json::from_slice(&out.stdout).expect("compose config is JSON"))
}

#[test]
fn shared_edge_overlay_runs_no_connector_and_puts_the_app_on_edge_echo_as_echo_app() {
    // The devnet runs one connector, the relay's at https://toonprotocol.dev/ilp, which
    // reaches the app over edge-echo as http://echo-app:3000.
    let Some(merged) = compose_config(&["docker-compose.yml", "docker-compose.shared-edge.yml"])
    else {
        return;
    };
    let services = merged["services"].as_object().expect("services");
    let names: Vec<&str> = services.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        ["echo-merchant"],
        "only the app runs under the overlay"
    );
    let app = &services["echo-merchant"];
    assert!(
        app["networks"]["default"].is_object(),
        "app keeps the default network"
    );
    assert_eq!(
        app["networks"]["edge-echo"]["aliases"],
        serde_json::json!(["echo-app"])
    );
    assert!(
        app.get("ports").is_none(),
        "the app is never published on the host"
    );
    assert_eq!(
        merged["networks"]["edge-echo"]["external"],
        serde_json::json!(true)
    );

    // Standalone (no overlay): the bundle's own connector still runs, on its loopback port.
    let base = compose_config(&["docker-compose.yml"]).unwrap();
    let connector = &base["services"]["connector"];
    assert_eq!(
        connector["ports"][0]["published"],
        serde_json::json!("4000")
    );
    assert!(base["services"]["echo-merchant"]["networks"]
        .get("edge-echo")
        .is_none());
}
