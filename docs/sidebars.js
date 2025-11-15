/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */

// @ts-check

const apiSidebar = require('./docs/api/sidebar.ts').default;

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  tutorialSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      items: [
        'getting-started/installation',
        'getting-started/quickstart',
        'getting-started/authentication',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      items: [
        'guides/embedding-text',
        'guides/caching',
        'guides/rate-limits',
      ],
    },
    // Add API Reference section with auto-generated API docs
    {
      type: 'category',
      label: 'API Reference',
      items: apiSidebar,
    },
  ],
};

module.exports = sidebars;
