# rwatch

`rwatch` is a command-line utility written in Rust that allows you to run a command repeatedly and watch its output. It's a Rust re-implementation of the classic Unix `watch` command.

## Features

- Run a given command repeatedly
- Clear screen between command runs
- Customizable interval for command execution
- Handle user interruption gracefully
- Cross-platform

## Installation

### Building from source

1. Make sure you have Rust installed. If not, install Rust using rustup:

    ```sh
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. Clone this repository:

    ```sh
    git clone https://github.com/davidhfrankelcodes/rwatch.git
    cd rwatch
    ```

3. Build and install `rwatch`:

    ```sh
    cargo build --release
    cargo install --path .
    ```

4. The `rwatch` command should now be available. Try running `rwatch --help` for usage information.

## Usage

```sh
rwatch <command> [interval]
```

### Example
To watch the contents of a directory change, you might use:

```sh
rwatch "ls -l" 1
```

## Contributing
Contributions to `rwatch` are welcome! Please read the contributing guidelines before submitting a pull request.

## License
`rwatch` is licensed under the [MIT License](https://opensource.org/license/mit).
