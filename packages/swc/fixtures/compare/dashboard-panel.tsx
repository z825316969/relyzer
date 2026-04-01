import React, { memo } from 'react';

const Footer = memo(function Footer({ value }) {
  const badge = value.badge;
  return <FooterWidget badge={badge} />;
});
