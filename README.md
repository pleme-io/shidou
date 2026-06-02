# shidou (始動)

App bootstrap for pleme-io Rust binaries: tracing initialization, CLI dispatch,
and config loading wired through one typed entrypoint.

`shidou` collapses the boilerplate every pleme-io binary repeats at startup —
`tracing_subscriber` setup (env-filter + JSON), `clap` dispatch, and
[shikumi](https://github.com/pleme-io/shikumi)-backed typed config loading —
into a single call so each tool declares *what* it does, not *how* it boots.

## Usage

```toml
[dependencies]
shidou = "0.1"
```

## License

MIT
