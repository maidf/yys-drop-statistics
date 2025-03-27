yys掉落统计  

devenv shell - 创建环境  

rust环境配置  

tauri环境配置


# 自动创建nix构建文件
# Check your Yarn version
yarn --version

# Upgrade to the latest version, if necessary
yarn set version berry

# Install the plugin
yarn plugin import https://raw.githubusercontent.com/stephank/yarn-plugin-nixify/main/dist/yarn-plugin-nixify.js

# Run Yarn as usual
yarn

# Build your project with Nix
nix-build


nix shell github:cargo2nix/cargo2nix

cargo2nix