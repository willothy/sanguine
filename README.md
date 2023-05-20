# sanguine

Create dynamic, performant TUI applications in Rust.

Built on top of [Termwiz](https://docs.rs/termwiz)' `BufferedTerminal`.

## Features:

- Dynamic, Tree-based layout
- Extensible widget trait
- Global keyboard events
- Focus handling
- Focus-based keyboard events
- Mouse click and hover events

## Demo

<!-- https://github.com/willothy/sanguine/assets/38540736/ccaeff03-fa50-4d4f-b070-f94d8e212097 -->

https://github.com/willothy/sanguine/assets/38540736/61a2ff5d-8284-437b-b31a-24045d133f3f



### Usage

```sh

$ git clone git@github.com:willothy/sanguine.git

$ cd sanguine

$ cargo run --example demo

```

### Demo keymaps:

- <kbd>Control</kbd> + <kbd>q</kbd>: Quit
- <kbd>Shift</kbd> + <kbd>Tab</kbd>: Cycle focus
- <kbd>Shift</kbd> + <kbd>Up/Down/Left/Right</kbd>: Switch focus by direction

Menu:

- <kbd>Up/Down/Left/Right</kbd>: Switch menu item
- <kbd>Enter</kbd>: Select menu item
