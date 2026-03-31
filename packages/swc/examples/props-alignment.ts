import { analyzeWithSwc } from '../dist/index';

const cases = {
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
  useRelyzerDirective: `
    import React from 'react';
    const Explicit = ({ a }) => {
      'use relyzer';
      const b = a + 1;
      return <Panel a={a} b={b} />;
    };
  `,
};

(async () => {
  for (const [name, code] of Object.entries(cases)) {
    const result = await analyzeWithSwc(code);
    console.log(`\n## ${name}`);
    console.log(JSON.stringify(result.components, null, 2));
  }
})();
