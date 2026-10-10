{
  description = "ruwa - lightweight WhatsApp desktop client for Linux (Gelectron)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      ruwa =
        { pkgs }:
        pkgs.rustPlatform.buildRustPackage {
          pname = "ruwa";
          version = "0.1.0";
          src = nixpkgs.lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
            nodejs
            imagemagick
          ];

          buildInputs = with pkgs; [
            webkitgtk_4_1
            libsoup_3
            gtk3
            glib
            glib-networking
            gsettings-desktop-schemas
            cairo
            pango
            gdk-pixbuf
            atk
            librsvg
            libxkbcommon
            wayland
            libX11
            libXi
            libXcursor
            libXrandr
            libXfixes
            libXdamage
            libXcomposite
            libXext
            xdotool
            openssl
          ];

          postInstall = ''
            mv $out/bin/gelectron $out/bin/.ruwa-unwrapped
            mkdir -p $out/share/ruwa
            cp -r ${./src/electron} $out/share/ruwa/compat
            cp -r ${./app} $out/share/ruwa/app
            ln -s ../share/ruwa/compat $out/bin/compat
            makeWrapper $out/bin/.ruwa-unwrapped $out/bin/ruwa \
              --prefix PATH : ${
                pkgs.lib.makeBinPath [
                  pkgs.nodejs
                  pkgs.xdg-utils
                  pkgs.bubblewrap
                  pkgs.xdg-dbus-proxy
                ]
              } \
              --set GIO_MODULE_DIR "${pkgs.glib-networking}/lib/gio/modules" \
              --set GIO_EXTRA_MODULES "${pkgs.glib-networking}/lib/gio/modules" \
              --set GSETTINGS_SCHEMA_DIR "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}/glib-2.0/schemas" \
              --prefix XDG_DATA_DIRS : "${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk3}/share" \
              --unset WAYLAND_DISPLAY \
              --add-flags "$out/share/ruwa/app"
            install -Dm644 ${./data/ruwa.desktop} $out/share/applications/ruwa.desktop
            for size in 16 24 32 48 64 128 256 512; do
              mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
              magick ${./data/icons/ruwa.png} -resize ''${size}x''${size} $out/share/icons/hicolor/''${size}x''${size}/apps/ruwa.png
            done
          '';

          meta = {
            mainProgram = "ruwa";
            description = "Lightweight WhatsApp desktop client for Linux (Gelectron)";
            license = nixpkgs.lib.licenses.mit;
            platforms = nixpkgs.lib.platforms.linux;
          };
        };
    in
    {
      packages = forAllSystems (pkgs: {
        default = ruwa { inherit pkgs; };
        "ruwa" = ruwa { inherit pkgs; };
      });

      overlays.default = final: _prev: {
        "ruwa" = ruwa { pkgs = final; };
      };

      apps = nixpkgs.lib.genAttrs systems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/ruwa";
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            pkg-config
            mold
            nodejs
            webkitgtk_4_1
            libsoup_3
            gtk3
            glib
            cairo
            pango
            gdk-pixbuf
            atk
            librsvg
            libxkbcommon
            wayland
            libX11
            libXi
            libXcursor
            libXrandr
            libXfixes
            libXdamage
            libXcomposite
            libXext
            xdotool
            openssl
            glib-networking
            gsettings-desktop-schemas
            bubblewrap
            xdg-dbus-proxy
          ];
          shellHook = ''
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
            export CARGO_INCREMENTAL=1
            export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
            export XDG_DATA_DIRS="$XDG_DATA_DIRS:${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk3}/share"
            export GSETTINGS_SCHEMA_DIR="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}/glib-2.0/schemas"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
            echo "ruwa dev shell ready. Run: cargo build -p gelectron"
          '';
        };
      });
    };
}
