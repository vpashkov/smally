import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

const sidebar: SidebarsConfig = {
  apisidebar: [
    {
      type: "category",
      label: "embeddings",
      link: {
        type: "doc",
        id: "api/embeddings",
      },
      items: [
        {
          type: "doc",
          id: "api/create-embedding-handler",
          label: "Create text embeddings",
          className: "api-method post",
        },
      ],
    },
    {
      type: "category",
      label: "health",
      link: {
        type: "doc",
        id: "api/health",
      },
      items: [
        {
          type: "doc",
          id: "api/root-handler",
          label: "API information endpoint",
          className: "api-method get",
        },
        {
          type: "doc",
          id: "api/health-handler",
          label: "Health check endpoint",
          className: "api-method get",
        },
      ],
    },
  ],
};

export default sidebar.apisidebar;
