# @relyzer/swc

A Rust + SWC powered analyzer package for `relyzer`.

## Status

This package is now able to:

- compile Rust analyzer logic to Wasm
- parse TS / TSX modules with SWC
- collect import metadata
- detect component-like functions
- extract a first batch of observed metadata (`var`, `dep`, `attr`)

Current scope is a **read-only analyzer**. It does **not** yet replace the Babel plugin's runtime code injection behavior.

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

## Output shape

The SWC analyzer aligns with the Babel analyzer's metadata model as closely as possible:

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
2. Introduce `@relyzer/swc` for static metadata analysis.
3. Gradually port more Babel-side analysis logic into SWC.
4. Evaluate whether runtime injection should later be reimplemented in SWC or kept as a separate layer.
