module.exports = {
  apps : [{
    name: 'xivanalysis',
    script: 'build/index.js',
    node_args: '--max-http-header-size=20000',
    env: {
      NODE_ENV: 'development'
    },
    env_production : {
      NODE_ENV: 'production'
    }
  }],
};
