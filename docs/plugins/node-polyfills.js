module.exports = function (context, options) {
  return {
    name: 'node-polyfills-plugin',
    configureWebpack(config, isServer) {
      if (!isServer) {
        return {
          resolve: {
            fallback: {
              buffer: require.resolve('buffer/'),
              url: require.resolve('url/'),
              stream: false,
              http: false,
              https: false,
              zlib: false,
              path: false,
              fs: false,
            },
          },
        };
      }
      return {};
    },
  };
};
