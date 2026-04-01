import React, { memo, useCallback, useMemo } from 'react';

export const MemoCard = memo(({ count, title, actions }) => {
  const doubled = useMemo(() => count * 2, [count]);
  const onSelect = useCallback(() => actions.select(title), [actions, title]);
  const view = {
    doubled,
    title,
  };

  return <CardView value={doubled} title={title} onSelect={onSelect} view={view} />;
});
