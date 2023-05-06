# sanguine

Create dynamic, performant TUI applications in Rust.

Built on top of [Termwiz](https://docs.rs/termwiz)' `BufferedTerminal`.

> **Note**<br>
> Sanguine is not quite ready for use.<br>
> You can run the `demo` example if you want to try it out.<br>
<br/>

> The demo is from the old version (in the `old` branch)<br>
> The rewritten version features a new, dynamic layout management system, but doesn't have floating windows or any builtin widgets yet.

## Goals:

- [x] Floating windows
  - [ ] Render optimizations
- [x] Horizontal and vertical splits
- [x] Global keyboard events
- [x] Extensible widget trait
- [x] Conventient layout primitives
- [ ] Focus
- [ ] Focused-widget keyboard events
- [ ] Mouse events

## Demo

Watch in fullscreen, the lines don't render properly in a small viewport.

[FloatingDemo.webm](https://user-images.githubusercontent.com/38540736/231884015-44bb77ce-2111-4d92-b463-b6a02b29be8b.webm)

## Demo usage

```sh

$ git clone git@github.com:willothy/sanguine.git

$ cd sanguine

$ cargo run --example demo

```

### Demo keymaps:

- `<C-q>`: quit
- `<Tab>`: switch layout (not working with new UI manager yet)
- `<Shift-Tab>`: switch current float
- `<Up>`: move the current float up
- `<Down>`: move the current float down
- `<Left>`: move the current float left
- `<Right>`: move the current float right
- `<Shift-Up>`: resize the current float
- `<Shift-Down>`: resize the current float
- `<Shift-Left>`: resize the current float
- `<Shift-Right>`: resize the current float
