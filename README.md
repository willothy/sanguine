# sanguine

Create dynamic, performant TUI applications in Rust.

Sanguine was created from the need for a library tailored for _complex_ TUI applications such as text editors and terminal multiplexers. The Rust ecosystem contains many great crates for building TUI apps, but many are geared towards small dashboard-like apps and implement immediate-mode rendering or struggle with mouse events.

Sanguine implements a tree-based layout API that can be updated at runtime, with a custom constraint algorithm geared towards rendering to the terminal. Layout results are cached between renders for performance, and are only recomputed when the layout is changed. Widgets can be nested and mouse events are handled properly for widgets at any depth - widgets only need to handle mouse events based on local position. Widgets can optionally specify a cursor location to allow for implementations of text editor windows and more.

It is built on top of [Termwiz](https://docs.rs/termwiz)' `BufferedTerminal`, which optimizes terminal writes to maximize performance.

## Features:

- Dynamic, Tree-based layout API
- Extensible widget trait
- First-class mouse events support
  - Automatic propagation
  - Hover and click support
- Global and widget-local event handlers
- Generic API
  - Custom user event type for message passing
  - Custom state type for core app state
- Focus
  - Switch focus by direction or directly

## Demo

<!-- https://github.com/willothy/sanguine/assets/38540736/ccaeff03-fa50-4d4f-b070-f94d8e212097 -->

[demo](https://github.com/willothy/sanguine/assets/38540736/61a2ff5d-8284-437b-b31a-24045d133f3f)

<details>
<summary>Demo Usage</summary>

```sh

$ git clone git@github.com:willothy/sanguine.git

$ cd sanguine

$ cargo run --example demo

```

Keymaps:

- <kbd>Control</kbd> + <kbd>q</kbd>: Quit
- <kbd>Shift</kbd> + <kbd>Tab</kbd>: Cycle focus
- <kbd>Shift</kbd> + <kbd>Up/Down/Left/Right</kbd>: Switch focus by direction
- <kbd>Up/Down/Left/Right</kbd>: Switch menu item
- <kbd>Enter</kbd>: Select menu item

</details>
