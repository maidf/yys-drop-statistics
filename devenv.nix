{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  unpkgs = import inputs.unpkgs { system = pkgs.stdenv.system; };
in
{

  # https://devenv.sh/basics/
  env.GREET = "devenv";
  packages = with unpkgs; [
    sqlite
    cargo-watch

    # tauri所需依赖
    pkg-config
    gobject-introspection
    cargo-tauri
    at-spi2-atk
    atkmm
    cairo
    gdk-pixbuf
    glib
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    openssl
  ];

  # https://devenv.sh/packages/
  #   packages = lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk; [
  #     frameworks.Security
  #   ]);
  # https://devenv.sh/lainguages/
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-src"
      "rust-std"
      "rust-docs"
    ];
  };
  languages.javascript = {
    enable = true;
    package = unpkgs.nodejs-slim;
    pnpm.enable = true;
    pnpm.package = unpkgs.pnpm;
  };

  services.nginx = {
    enable = true;
    package = unpkgs.nginx;
    httpConfig = "
      server {
          listen 9901;
          server_name localhost;

          root /usr/share/nginx/html;
          index index.html;

          location / {
              try_files $uri $uri index.html;
          }

          location /api {
              proxy_pass http://127.0.0.1:9909;
          }

          location ~* \.(js|css|png)$ {
              expires 7d;
          }
      }
    ";
  };

  # https://devenv.sh/processes/
  # 项目启动流程
  processes = {
    backend = {
      exec = "cd back && cargo watch -x run";
      #   process-compose.depends_on = {
      #     opensearch.condition = "process_healthy";
      #     postgres.condition = "process_healthy";
      #   };
    };
    frontend = {
      exec = "cd fr && pnpm dev";
    };
  };

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  # bash脚本
  scripts.hello.exec = ''
    sqlite3 --version
    echo hello from $GREET
  '';

  #   scripts.upng.exec = ''
  #     cd fr
  #     pnpm build
  #   '';

  enterShell = ''
    hello
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
