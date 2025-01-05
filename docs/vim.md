# Vim Commands

Vim bindings are the default in the TUI. Here's an (in-progress) list of the commands we plan to support, or support already:

## Supported
Motion:
- h
- j
- k
- l
- w/W
- b/B
- e
- 0
- $
- f/F
- t/T

Edit/Insert:
- i/I
- a/A
- d/D
- c/C
- x
- y
- r

Text objects (iw, etc.)

Visual mode

Undo

## In-Progress/Future
- redo (proper keybind, functionality works with `R`)
- / for search (waiting on API)

## Not Planned (atm)
- Macros
- g* commands (excluding gg)
- n/N 
- %