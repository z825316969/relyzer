# @relyzer/swc

A Rust + SWC powered analyzer package for `relyzer`.

## Status

`@relyzer/swc` is currently a **read-only analyzer**. It does **not** replace the Babel plugin's runtime injection behavior.

What it does today:

- compiles Rust analyzer logic to Wasm
- parses TS / TSX modules with SWC
- collects import metadata
- detects component-like functions using the same current rules as Babel
- extracts observed metadata for:
  - `var`
  - `dep`
  - `attr`

## Babel parity

The current SWC read-only analyzer is verified against the Babel analyzer with **strict JSON equality** on a growing compare suite.

Covered parity scenarios include:

- uppercase component auto-detection
- `memo(() => {})`
- `memo(function () {})`
- object-pattern / nested destructuring props
- explicit `@component` comments
- `'use relyzer'` directives
- hook dependency extraction from `useMemo` / `useCallback`
- JSX attribute observation in realistic nested component trees

## Build

```bash
pnpm --filter @relyzer/swc run build:wasm
```

If you want the JS glue for Node usage:

```bash
wasm-bindgen packages/swc/target/wasm32-unknown-unknown/release/relyzer_swc.wasm \
  --out-dir packages/swc/dist/wasm \
  --target nodejs
```

## Usage

```ts
import { analyzeWithSwc } from '@relyzer/swc';

const result = await analyzeWithSwc(`
  import React, { useMemo } from 'react';

  /** @component */
  export function Demo({ count, title }) {
    const value = useMemo(() => count * 2, [count]);
    const info = { value };

    return <Widget label={title} data={info} />;
  }
`);

console.log(result.components);
```

## Compare Babel vs SWC

Use the built-in compare harness to verify that Babel and SWC produce identical read-only metadata:

```bash
pnpm --filter @relyzer/swc run compare
```

The compare script:

- runs the same source through Babel and SWC
- normalizes import / observed metadata ordering
- checks **strict JSON equality**
- includes both inline edge cases and real fixture files under `packages/swc/fixtures/compare`

## Output shape

The SWC analyzer aligns with the Babel analyzer's metadata model:

- `components[]`
  - `id`
  - `name`
  - `code`
  - `loc`
  - `observedList[]`
  - `shouldDetectCallStack`
- `imports[]`
  - `source`
  - `specifiers[]`

## Migration guide

### Babel version

The current Babel-based integration is centered around development-time transform / injection behavior.

### SWC version

The SWC package is better suited for:

- fast static analysis
- Rust/Wasm based metadata extraction
- future high-performance analyzer expansion

### Recommended migration path

1. Keep existing Babel injection behavior for runtime instrumentation.
2. Use `@relyzer/swc` for static metadata analysis.
3. Validate parity with `pnpm --filter @relyzer/swc run compare` when extending analyzer behavior.
4. Gradually port more Babel-side analysis logic into SWC.
5. Evaluate later whether runtime injection should be reimplemented in SWC or remain a separate layer.
