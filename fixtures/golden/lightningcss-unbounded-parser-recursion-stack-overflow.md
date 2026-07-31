# Unbounded recursion in selector, math-function and rule-nesting parsing overflows the stack

**Version tested: 1.0.0-alpha.72.** All line numbers are relative to this version.

lightningcss defines `ParserError::MaximumNestingDepth` at `src/error.rs:112` and enforces a depth limit of 500 for custom-property token trees at `src/properties/custom.rs:321-323`. Under that guard, `var()` and bare-parenthesis nesting survive depth 50,000.

Selector parsing, math-function parsing and rule nesting have no equivalent guard. `StyleSheet::parse` recurses without bound on those three paths until the stack is exhausted.

## Reproducer

```rust
let css: &'static str = Box::leak(
    format!("{}a{}{{color:red}}", ":is(".repeat(2000), ")".repeat(2000)).into_boxed_str()
);
let r = std::panic::catch_unwind(|| {
    StyleSheet::parse(css, ParserOptions::default()).is_ok()
});
println!("{:?}", r);   // never reached
```

The input is 10,012 bytes of syntactically valid CSS. stderr:

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

Exit code 134, in both debug and release profiles. This is the stack guard calling `abort()` rather than a panic, so `catch_unwind` does not contain it.

## Other inputs that reach the same abort

| Input | Repetitions |
|---|---|
| `:not(` | 1,984 |
| `a{` (nested style rules) | 3,460 |
| `@media screen{` | 2,476 |
| `@supports (display:grid){` | 2,476 |
| `calc(` | 5,500 |
| `min(` | 5,000 |

## Recursion paths involved

- unguarded recursion through `Calc::parse_sum`, `src/values/calc.rs:540`
- unguarded recursion through `SelectorList::parse`, `src/selector.rs:2338`
- rule nesting, reached from the entry point at `src/stylesheet.rs:124`

