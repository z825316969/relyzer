/* eslint-disable no-console */
import fs from 'fs';
import path from 'path';
import { transformSync } from '../../babel/node_modules/@babel/core';
import syntaxJsx from '../../babel/node_modules/@babel/plugin-syntax-jsx';
import relyzerBabelPluginModule from '../../babel/lib/index';
import { analyzeWithSwc, SwcAnalyzeResult, ComponentMetaData, ImportMeta, ObservedMeta } from './index';

const relyzerBabel = (relyzerBabelPluginModule as any).default || relyzerBabelPluginModule;

type NormalizedObservedMeta = Pick<ObservedMeta, 'name' | 'loc' | 'type'>;
type NormalizedComponent = Omit<ComponentMetaData, 'id' | 'name' | 'observedList'> & {
  observedList: NormalizedObservedMeta[];
};
type NormalizedResult = Omit<SwcAnalyzeResult, 'components'> & {
  components: NormalizedComponent[];
};

function astObjectToValue(node: any): any {
  switch (node?.type) {
    case 'ObjectExpression':
      return Object.fromEntries((node.properties ?? []).map((property: any) => {
        const key = property.key.type === 'Identifier' ? property.key.name : property.key.value;
        return [key, astObjectToValue(property.value)];
      }));
    case 'ArrayExpression':
      return (node.elements ?? []).map((item: any) => astObjectToValue(item));
    case 'StringLiteral':
    case 'NumericLiteral':
    case 'BooleanLiteral':
      return node.value;
    case 'NullLiteral':
      return null;
    case 'Identifier':
      if (node.name === 'undefined') return undefined;
      throw new Error(`Unsupported identifier in Babel meta AST: ${node.name}`);
    default:
      throw new Error(`Unsupported Babel meta AST node: ${node?.type ?? 'unknown'}`);
  }
}

const inlineCases: Record<string, string> = {
  identifierProps: `
    import React from 'react';
    export function Card(props) {
      const title = props.title;
      return <Panel value={props.value} title={title} />;
    }
  `,
  objectPatternProps: `
    import React from 'react';
    /** @component */
    export function Card({ title, value }) {
      const merged = { title, value };
      return <Panel title={title} value={merged} />;
    }
  `,
  noParamExplicit: `
    import React from 'react';
    /** @component */
    export function helper() {
      return <Panel />;
    }
  `,
  memoArrow: `
    import React, { memo, useMemo } from 'react';
    const MemoCard = memo(({ count }) => {
      const doubled = useMemo(() => count * 2, [count]);
      return <Panel value={doubled} count={count} />;
    });
  `,
  memoFunction: `
    import React, { memo } from 'react';
    const MemoFn = memo(function MemoFn(props) {
      const ready = props.ready;
      return <Panel ready={ready} />;
    });
  `,
  useRelyzerDirectiveExplicitOnly: `
    import React from 'react';
    const helper = ({ a }) => {
      'use relyzer';
      const b = a + 1;
      return <Panel a={a} b={b} />;
    };
  `,
  hookDependencyMatrix: `
    import React, { useCallback, useMemo } from 'react';
    /** @component */
    export function Dashboard(props) {
      const { user, count } = props;
      const summary = useMemo(() => ({ user, count }), [user, count]);
      const onOpen = useCallback(() => user.open(count), [user, count]);
      return <Panel summary={summary} onOpen={onOpen} user={user} />;
    }
  `,
};

function sortImports(imports: ImportMeta[]): ImportMeta[] {
  return [...imports]
    .map((item) => ({
      source: item.source,
      specifiers: [...item.specifiers].sort(),
    }))
    .sort((a, b) => a.source.localeCompare(b.source) || a.specifiers.join(',').localeCompare(b.specifiers.join(',')));
}

function sortObservedList(observedList: NormalizedObservedMeta[]): NormalizedObservedMeta[] {
  return [...observedList].sort((a, b) => (
    a.type.localeCompare(b.type)
    || a.name.localeCompare(b.name)
    || a.loc.localeCompare(b.loc)
  ));
}

function normalizeResult(result: SwcAnalyzeResult): NormalizedResult {
  const deduped = new Map<string, ComponentMetaData>();

  for (const component of result.components) {
    const key = `${component.loc ?? ''}::${component.code}`;
    const existing = deduped.get(key);
    if (!existing || component.shouldDetectCallStack) {
      deduped.set(key, component);
    }
  }

  return {
    ok: result.ok,
    errors: [...result.errors],
    imports: sortImports(result.imports),
    components: [...deduped.values()]
      .map((component) => ({
        code: component.code,
        loc: component.loc,
        shouldDetectCallStack: component.shouldDetectCallStack,
        observedList: sortObservedList(component.observedList.map((item) => ({
          name: item.name,
          loc: item.loc,
          type: item.type,
        }))),
      }))
      .sort((a, b) => (
        (a.loc ?? '').localeCompare(b.loc ?? '')
        || a.code.localeCompare(b.code)
      )),
  };
}

function collectFixtureCases(): Record<string, string> {
  const fixtureDir = path.resolve(__dirname, '../fixtures/compare');
  if (!fs.existsSync(fixtureDir)) {
    return {};
  }

  return Object.fromEntries(
    fs.readdirSync(fixtureDir)
      .filter((file) => file.endsWith('.tsx') || file.endsWith('.ts') || file.endsWith('.jsx') || file.endsWith('.js'))
      .sort()
      .map((file) => [
        `fixture:${file}`,
        fs.readFileSync(path.join(fixtureDir, file), 'utf8'),
      ]),
  );
}

function extractBabelResult(code: string, filename: string): SwcAnalyzeResult {
  const result = transformSync(code, {
    filename,
    babelrc: false,
    configFile: false,
    ast: true,
    code: true,
    plugins: [
      syntaxJsx,
      [relyzerBabel, { autoDetect: true }],
    ],
  });

  const output = result?.code ?? '';
  const imports: ImportMeta[] = [];
  const metas: ComponentMetaData[] = [];
  const programBody = result?.ast?.program.body ?? [];

  for (const node of programBody as any[]) {
    if (node.type !== 'ImportDeclaration' || node.source?.value === '@relyzer/runtime') {
      continue;
    }

    imports.push({
      source: node.source.value,
      specifiers: (node.specifiers ?? []).map((specifier: any) => specifier.local.name),
    });
  }

  const stack: any[] = [...programBody];
  while (stack.length) {
    const node = stack.pop();
    if (!node || typeof node !== 'object') {
      continue;
    }

    if (
      node.type === 'CallExpression'
      && node.callee?.type === 'Identifier'
      && /^_?useRelyzer$/.test(node.callee.name)
      && node.arguments?.[0]?.type === 'ObjectExpression'
    ) {
      metas.push(astObjectToValue(node.arguments[0]) as ComponentMetaData);
    }

    for (const value of Object.values(node)) {
      if (Array.isArray(value)) {
        stack.push(...value);
      } else if (value && typeof value === 'object') {
        stack.push(value);
      }
    }
  }

  return {
    ok: true,
    imports,
    components: metas,
    errors: [],
  };
}

async function main() {
  let hasDiff = false;
  const cases = {
    ...inlineCases,
    ...collectFixtureCases(),
  };

  for (const [name, code] of Object.entries(cases)) {
    const babelResult = normalizeResult(extractBabelResult(code, `${name}.jsx`));
    const swcResult = normalizeResult(await analyzeWithSwc(code));
    const same = JSON.stringify(babelResult) === JSON.stringify(swcResult);

    console.log(`\n=== ${name} ===`);
    if (same) {
      console.log('PASS');
      continue;
    }

    hasDiff = true;
    console.log('FAIL');
    console.log('\n[Babel]');
    console.log(JSON.stringify(babelResult, null, 2));
    console.log('\n[SWC]');
    console.log(JSON.stringify(swcResult, null, 2));
  }

  if (hasDiff) {
    process.exitCode = 1;
    return;
  }

  console.log('\nAll compare cases passed with strict JSON equality.');
}

void main();
