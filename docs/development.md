# Development

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The app is one axum router and holds no keys and no state.
[`CONTEXT.md`](../CONTEXT.md) is the glossary.

`.github/workflows/publish-image.yml` builds the image and pushes it to
`ghcr.io/toon-protocol/echo-merchant`: `release` and `latest` on every push to `main`, an
immutable sha tag on every build, and semver tags on a `v*` tag.
[`deploy/README.md`](../deploy/README.md) runs it.
