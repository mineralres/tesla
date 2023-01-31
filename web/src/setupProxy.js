const { createProxyMiddleware } = require('http-proxy-middleware');

module.exports = function (app) {
    app.use(createProxyMiddleware('/api/tesla/*', { target: 'http://localhost:3600/' }));
};