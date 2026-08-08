# Contributing to The Yahoo Finance API Crate

Thank you for your interest in contributing! This document provides simple guidelines for getting involved.

## Development

### Building

```sh
cargo build
```

### Testing

Run all tests:
```sh
cargo test
```

Run specific example:
```sh
cargo run --example get_quote
```

### Code Style
- Follow standard Rust naming conventions
- Use `cargo fmt` to format code
- Run `cargo clippy` to check for common mistakes

### Using a Dev Container

This project includes an optional Dev Container configuration with all necessary tools pre-installed (Rust, Cargo, Git, and VS Code extensions). Using it is entirely optional, but can simplify setup.

**VS Code:**
1. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) or the entire [Remote Development extension pack](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.vscode-remote-extensionpack)
2. Open the project folder in VS Code
3. Click "Reopen in Container" when prompted, or press `Ctrl+Shift+P` and select "Dev Containers: Reopen in Container"

**Other IDEs:**
- Check your IDE's documentation for Dev Container support (e.g., JetBrains IDEs, Vim with proper plugins)
- Alternatively, you can work with containers using Docker directly with the `.devcontainer/devcontainer.json` configuration and the [Dev Container CLI](https://github.com/devcontainers/cli).

## Submitting Changes

1. Create a new branch for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes and commit with clear messages
3. Push to your fork and submit a pull request with a detailed description of your changes
4. Ensure all tests pass and CI checks succeed

## License

By contributing, you agree that your contributions will be licensed under both Apache 2.0 and MIT licenses, consistent with the project.

## Questions?

Feel free to open an issue to discuss any questions or suggestions.
