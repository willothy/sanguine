# sanguine

Create dynamic, performant TUI applications in Rust.

Built on top of [Termwiz](https://docs.rs/termwiz)' `BufferedTerminal`.

> **Note**<br>
> Sanguine is not quite ready for use.<br>
> You can run the `demo` example if you want to try it out.<br>

## Goals:

- [x] Tree-based layout
- [x] Global keyboard events
- [x] Extensible widget trait
- [x] Focus
- [x] Focused-widget keyboard events
- [ ] Mouse events

## Demo

```sh

$ git clone git@github.com:willothy/sanguine.git

$ cd sanguine

$ cargo run --example demo

```

### Demo keymaps:

- `<C-q>`: quit
- `<S-Tab>`: switch window
