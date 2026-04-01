import React from 'react';

export const helperWithDirective = ({ a, b }) => {
  'use relyzer';
  const total = a + b;
  return <Panel a={a} total={total} />;
};
