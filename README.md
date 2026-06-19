# thefug

## Install

```sh
cargo install --path .
```

Then add the following to your shell rc file:

```sh
# ~/.bashrc
eval "$(thefug init bash)"
```

```sh
# ~/.zshrc
eval "$(thefug init zsh)"
```

```fish
# ~/.config/fish/config.fish
thefug init fish | source
```

`init` takes the shell name because it can't reliably detect the shell sourcing its
output — `$SHELL` reports the parent process's login shell, not the running shell, so
detection breaks when started from a non-interactive shell.

Open a new shell. Type a wrong command, then type `fug`:

```sh
$ gti pll
zsh: command not found: gti
$ fug
Running: git pull
...
```

## Development

```sh
cargo build
eval "$(./target/debug/thefug init zsh)"   # zsh/bash
./target/debug/thefug init fish | source   # fish
```

The eval embeds the absolute path of the binary you ran `init` from, so you don't need to put `target/debug` on PATH. Rebuild + re-eval (in a new shell, or just re-run the eval) to pick up changes.

To iterate on the algorithm without polluting your history:

```sh
./target/debug/thefug simulate --history tests/fixtures/git_repeated_typo.txt --print "gti pll"
```
