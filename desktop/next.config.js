const { version } = require('./package.json');

const demo = process.env.NEXT_PUBLIC_DEMO === '1';

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  env: {
    NEXT_PUBLIC_APP_VERSION: version,
  },
  images: {
    unoptimized: true,
  },
  trailingSlash: true,
  // Disable server-side features for Tauri
  reactStrictMode: true,
  // The website demo is served from /demo/ on the landing site.
  ...(demo ? { basePath: '/demo' } : {}),
};

module.exports = nextConfig;
