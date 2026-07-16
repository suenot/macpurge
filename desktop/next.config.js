const { version } = require('./package.json');

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
};

module.exports = nextConfig;
