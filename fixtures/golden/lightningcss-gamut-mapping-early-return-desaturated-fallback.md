# Gamut mapping returns at the first under-JND chroma, producing desaturated sRGB fallbacks

**Version tested: lightningcss 1.0.0-alpha.72.** All line numbers below are relative to this version.

## Summary

`CssColor`'s gamut mapping returns as soon as it finds a chroma whose clipped form is under the JND threshold. The CSS Color 4 algorithm continues searching upward from that point and converges on the *largest* chroma still within the JND. The result is an sRGB fallback that is consistently less saturated than the specification produces.

## Reproducer

```rust
let old = Targets {
    browsers: Some(Browsers { chrome: Some(80 << 16), ..Default::default() }),
    ..Default::default()
};
let mut ss = StyleSheet::parse("a{color:lab(98.2504% -7.697 88.7581)}", ParserOptions::default()).unwrap();
ss.minify(MinifyOptions { targets: old.clone(), ..Default::default() }).unwrap();
println!("{}", ss.to_css(PrinterOptions { minify: true, targets: old, ..Default::default() }).unwrap().code);
```

Output:

```
a{color:#fffbac;color:lab(98.2504% -7.697 88.7581)}
```

| Input | lightningcss | colorjs.io 0.7.1, `method: "css"` |
|---|---|---|
| `lab(98.2504% -7.697 88.7581)` | `#fffbac` | `#fffc44` |
| `color(prophoto-rgb 0.9784 0.9925 0.3037)` | `#fffea7` | `#ffff6d` |
| `lch(94.4698% 111.9227 117.6737deg)` | `#bcff61` | `#b0ff00` |

The three expected values above are the output of **colorjs.io 0.7.1 with `method: "css"`**, run on the same inputs:

```js
new Color("lab(98.2504% -7.697 88.7581)")
    .to("srgb")
    .toGamut({ method: "css", space: "srgb" })
    .toString({ format: "hex" });
// #fffc44
```

`method: "css"` is colorjs.io's implementation of the §14.2.2 algorithm: `JND = 0.02`, `ε = 0.0001`, binary search on OKLCh chroma with `deltaEOK` as the local MINDE test.

Separately, across 2,719 differential cases against a transcription of the §14.2.2 pseudocode, 104 exceed the 0.02 JND the algorithm is built to respect. Worst observed deltaEOK is 0.0921, 4.6x the JND. Every deviation measured is in the desaturating direction.

## What the specification says

CSS Color 4 §14.2.2, "Sample Pseudocode for the Binary Search Gamut Mapping with Local MINDE":

https://www.w3.org/TR/css-color-4/#pseudo-binsearch

The algorithm contains an unconditional immediate return for `deltaE < JND`, but that is the pre-loop check, before the binary search begins. The in-loop occurrence is conditional: the specification returns `clipped` only when `(JND - E) < epsilon`, and otherwise sets `min_inGamut = false`, sets `min = chroma`, and continues searching upward.

lightningcss returns at the first in-loop chroma where `delta_e < JND` (`src/values/color.rs:3370-3371`), omitting the `min_inGamut` flag entirely. Because the search stops early, the chosen chroma is lower than or equal to the one the specification converges on. It is equal where the first in-loop candidate already satisfies `(JND - E) < epsilon`, and lower in all 2,719 cases compared.

colorjs.io 0.7.1 carries that flag in its own source: `min_inGamut` at `src/toGamut.js:307`, with `JND = 0.02` at `:245` and `ε = 0.0001` at `:246`. So the continuation lightningcss omits is present both in the specification text and in the implementation the values above came from.

## A note on the section reference

The code cites this algorithm at `src/values/color.rs:3330` using the anchor `#binsearch`. That anchor no longer resolves. §13.2, where the algorithm previously sat, is now "Interpolating with Missing Components". The algorithm has moved to §14.2.2 under the anchor `#pseudo-binsearch`.

## Affected code

- `src/values/color.rs:3358-3375`, the search loop
- `src/values/color.rs:3370-3371`, the early return
- `src/values/color.rs:3330`, the specification citation
