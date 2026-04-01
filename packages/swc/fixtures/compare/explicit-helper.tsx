import React, { useMemo } from 'react';

/** @component */
export function helper() {
  const options = useMemo(() => ({ ready: true }), []);
  return <Widget options={options} />;
}
