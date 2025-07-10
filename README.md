# rwatch

A modern, cross-platform Rust alternative to `watch`, with color, diff, and more.

## Features
- Periodically runs a command and displays its output fullscreen
- Highlights differences between runs (with `-d`/`--differences`)
- Optionally keeps all changes since first run (`-d=permanent`)
- Supports ANSI color output (`-c`/`--color`)
- Beeps on command failure (`-b`/`--beep`)
- Exits on error, output change, or unchanged output for N cycles
- Customizable interval (via `-n`, `--interval`, or `WATCH_INTERVAL` env)
- No-title, no-wrap, and direct exec modes

## Usage

```sh
rwatch [OPTIONS] -- command [args...]
```

### Common Options
- `-n, --interval <seconds>`: Set update interval (default: 2, or `$WATCH_INTERVAL`)
- `-d, --differences[=permanent]`: Highlight output differences; keep all changes with `=permanent`
- `-c, --color`: Show ANSI color sequences
- `-b, --beep`: Beep if command exits non-zero
- `-e, --errexit`: Freeze on error and exit after key press
- `-g, --chgexit`: Exit when output changes
- `-q, --equexit <cycles>`: Exit when output does not change for N cycles
- `-t, --no-title`: Hide header
- `-w, --no-wrap`: Disable line wrapping
- `-x, --exec`: Pass command directly (no shell)

### Examples

- Watch a directory listing, highlighting changes:
  ```sh
  rwatch -d -- ls -l
  ```
- Run a command every 5 seconds:
  ```sh
  rwatch -n 5 -- date
  ```
- Watch a command, beep on error:
  ```sh
  rwatch -b -- make test
  ```
- Use a custom interval from the environment:
  ```sh
  WATCH_INTERVAL=10 rwatch -- git status
  ```

## Platform Notes
- On Windows, the default shell is `sh` (if available). Use `-x` for direct exec.
- Output is fullscreen; press Ctrl+C to exit.

## License
MIT
