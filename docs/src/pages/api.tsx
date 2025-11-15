import React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

export default function ApiDocs() {
  return (
    <Layout
      title="API Reference"
      description="Interactive API documentation for Smally Embeddings API"
      noFooter
    >
      <BrowserOnly fallback={<div>Loading...</div>}>
        {() => {
          const { API } = require('@stoplight/elements');
          require('@stoplight/elements/styles.min.css');

          return (
            <div style={{ height: 'calc(100vh - 60px)' }}>
              <API
                apiDescriptionUrl="/docs/openapi.json"
                router="hash"
                layout="sidebar"
                hideSchemas={false}
                hideInternal={false}
                hideExport={false}
                tryItCredentialsPolicy="include"
                tryItCorsProxy="https://cors.stoplight.io"
              />
            </div>
          );
        }}
      </BrowserOnly>
    </Layout>
  );
}
