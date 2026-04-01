import React from 'react';

/** @component */
export function Card({ title, value, meta: { kind }, items = [] }) {
  const merged = { title, value, kind };
  const [first] = items;
  return <Panel title={title} value={merged} kind={kind} first={first} />;
}
