# Befreak interpreter

[![dependency status](https://deps.rs/repo/github/PartyWumpus/befreak-interpreter/status.svg)](https://deps.rs/repo/github/PartyWumpus/befreak-interpreter)
[![Build Status](https://github.com/PartyWumpus/befreak-interpreter/workflows/CI/badge.svg)](https://github.com/PartyWumpus/befreak-interpreter/actions?workflow=CI)

This is a [befreak](http://tunes.org/~iepos/befreak.html) interpreter (an esoteric language from 2003) written in rust 🦀.

You can try it out at <https://partywumpus.github.io/befreak-interpreter>.

The main gimmick of the language, beyond being 2D, is that it is fully reversible. 
To take advantage of this there's a `?` operator which will enter 'inverse mode', interpreting every operator from then on as its inverse. 
You can also press the reverse button at any time to switch direction and go into inverse mode, which will undo all previous operations until it gets back to the initial state.

Data is stored in one of two stacks, the main stack and the control stack. The main stack is where most operations work and is where data is written/read to/from. The control stack is used for storing the result of comparisons and branches for deciding which direction to turn and to make it possible to follow a branch in reverse.

## Instruction Reference

- `0-9`
  - XOR top item with a value 0 thru 9 (multidigit also works)
  - `[x] -> [x']`

- `(`
  - Push a zero onto the stack
  - `() -> [0]`

- `)`
  - Pop a zero from the stack
  - `[0] -> ()`
  - (Errors if you attempt to pop a non-zero value)

- `[`
  - Transfer the top of main stack to control stack

- `]`
  - Transfer the top of control stack to the main stack

- `$`
  - Swap the top item with the top of control stack

- `w`
  - Write the top item to stdout as a character
  - `[x] -> ()`

- `r`
  - Read a character from stdin to the top of stack
  - `() -> [x]`

- `'`
  - Increment the top item
  - `[x] -> [x + 1]`

- ``` ` ```
  - Decrement the top item
  - `[x] -> [x - 1]`

- `+`
  - Add the top item to the next item
  - `[y] [x] -> [y+x] [x]`

- `-`
  - Subtract the top item from the next item
  - `[y] [x] -> [y-x] [x]`

- `%`
  - Divide y by x, leaving a quotient and remainder
  - `[y] [x] -> [y/x] [y%x] [x]`

- `*`
  - Undo the effects of %, using multiplication
  - `[z] [y] [x] -> [z*x+y] [x]`

- `~`
  - Bitwise NOT the top item
  - `[x] -> [~x]`

- `&`
  - Bitwise AND top two items, XOR'ing to the third
  - `[z] [y] [x] -> [z^(y&x)] [y] [x]`

- `|`
  - Bitwise OR top two items, XOR'ing to the third
  - `[z] [y] [x] -> [z^(y|x)] [y] [x]`

- `#`
  - Bitwise XOR the top item to the next item
  - `[y] [x] -> [y^x] [x]`

- `{`
  - Rotate "y" to the left "x" bits
  - `[y] [x] -> [y'] [x]`

- `}`
  - Rotate "y" to the right "x" bits
  - `[y] [x] -> [y'] [x]`

- `!`
  - Toggle top of control stack (i.e., XOR it with 1)

- `=`
  - If y equals x, toggle top of control stack
  - `[y] [x] -> [y] [x]`

- `l`
  - If y is less than x, toggle top of control stack
  - `[y] [x] -> [y] [x]`

- `g`
  - If y is greater than x, toggle top of control stack
  - `[y] [x] -> [y] [x]`

- `s`
  - Swap the top two items
  - `[y] [x] -> [x] [y]`

- `d`
  - Dig the third item to the top
  - `[z] [y] [x] -> [y] [x] [z]`

- `b`
  - Bury the first item under the next two
  - `[z] [y] [x] -> [x] [z] [y]`

- `f`
  - Flip the order of the top three items
  - `[z] [y] [x] -> [x] [y] [z]`

- `c`
  - Swap the second and third items
  - `[z] [y] [x] -> [y] [z] [x]`

- `o`
  - "Over": dig copy of second item to the top
  - `[y] [x] -> [y] [x] [y]`

- `u`
  - "Under": the inverse of "over"
  - `[y] [x] [y] -> [y] [x]`

- `:`
  - Duplicate the top item
  - `[x] -> [x] [x]`

- `;`
  - Unduplicate the top two items
  - `[x] [x] -> [x]`

- `"`
   - Enter string mode
   - String mode will copy all values as their ascii equivalent onto the stack until the next `"` is reached
   - If you want to write `"` you need to just put `34` on the stack manually, like `@(34w`

- `?`
  - Toggle inverse mode
  - Inverse mode makes every operator become its inverse, like `(` becomes `)`
  - Many operators are their own inverses, including this one

- `@`
  - Halt. Also signals the entrance point for the program

- `\`
  - If going east or west, turn right; otherwise, turn left

- `/`
  - If going east or west, turn left; otherwise, turn right

- `>`    
  - If going north, go east and push 1 onto control stack (in inverse mode, push 0)
  - If going south, go east and push 0 onto control stack (in inverse mode, push 1)
  - If going west, pop control stack and go south if 0, north if 1. (opposite in inverse mode)
  - If going east, toggle top of control stack, toggle inverse mode, and go west.
- `<`
  - If going north, go west and push 0 onto control stack (in inverse mode, push 1)
  - If going south, go west and push 1 onto control stack (in inverse mode, push 0)
  - If going east, pop control stack and go north if 0, south if 1. (opposite in inverse mode)
  - If going west, toggle top of control stack, toggle inverse mode, and go east.
- `v`
  - If going east, go south and push 1 onto control stack (in inverse mode, push 0)
  - If going west, go south and push 0 onto control stack (in inverse mode, push 1)
  - If going north, pop control stack and go west if 0, east if 1. (opposite in inverse mode)
  - If going south, toggle top of control stack, toggle inverse mode, and go north.
- `^`
  - If going east, go north and push 0 onto control stack (in inverse mode, push 1)
  - If going west, go north and push 1 onto control stack (in inverse mode, push 0)
  - If going south, pop control stack and go east if 0, west if 1. (opposite in inverse mode)
  - If going north, toggle top of control stack, toggle inverse mode, and go south.

## Building from source

Run `nix develop` to get all the relevant dependencies for any of these steps.

### Running natively locally

Run `cargo run --release` to open it as a native egui app.

### Web Locally

0. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
0. Open `http://127.0.0.1:8080/index.html#dev` in a browser.

### Manual Web Deploy
0. Run `trunk build --release`.
0. Upload the `dist` directory to the hosting site.

## Updating egui

As of 2023, egui is in active development with frequent releases with breaking changes. [eframe_template](https://github.com/emilk/eframe_template/) will be updated in lock-step to always use the latest version of egui.

When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).

