# OSC 8 hyperlink text lost after an ST terminator

## Reproducer

```rust
use ansi_to_tui::IntoText;

let st = b"pre \x1b]8;;https://a.b\x1b\\LINK\x1b]8;;\x1b\\ post";
println!("{:?}", st.into_text().unwrap());
```

## Observed

The spans flatten to `"pre "`. The BEL-terminated form keeps the link text.
While the parser recognizes BEL as an OSC terminator, an ST-terminated
sequence consumes every byte to the end of the line.

## Expected

`"pre LINK post"`. ECMA-48 section 8.3.89 defines OSC as a control string
terminated by ST, and BEL is the xterm extension.

## Root cause

The parse at `src/parser.rs:273` ends the OSC command string only at BEL, so
an ST-terminated sequence consumes up to the next BEL or the end of the line,
including the visible text between them.
