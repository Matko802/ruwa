{
  description = "ruwa - lightweight WhatsApp desktop client for Linux (Tauri 2 + system WebKitGTK)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      ruwa =
        { pkgs }:
        let
          gstPlugins = with pkgs.gst_all_1; [
            gst-plugins-base
            gst-plugins-good
            gst-plugins-bad
            gst-plugins-ugly
            gst-libav
          ];
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "ruwa";
          version = "0.4.0";
          src = nixpkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            makeWrapper
            imagemagick
          ];
          buildInputs = with pkgs; [
            # Tauri 2 / WRY system webview (WebKitGTK 4.1, GTK3)
            webkitgtk_4_1
            gtk3
            libsoup_3
            cairo
            gdk-pixbuf
            glib
            pango
            atk
            librsvg
            libnotify
            libXtst
            libayatana-appindicator
            libdbusmenu
            openssl
            # Media capture (calls): GStreamer + audio plumbing
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-ugly
            gst_all_1.gst-libav
            pipewire
            libpulseaudio
            alsa-lib
            # TLS for WebKitGTK (libsoup) via GIO modules
            glib-networking
            # Settings app (GTK4)
            gtk4
            graphene
            gsettings-desktop-schemas
            adwaita-icon-theme
          ];
          postInstall = ''
            mkdir -p $out/share/applications $out/share/ruwa
            cp ${./data/ruwa.desktop} $out/share/applications/ruwa.desktop
            cp -r ${./mods} $out/share/ruwa/mods
            for size in 16 24 32 48 64 128 256 512; do
              mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
              ${pkgs.imagemagick}/bin/magick ${./data/icons/ruwa.png} -resize ''${size}x''${size} $out/share/icons/hicolor/''${size}x''${size}/apps/ruwa.png
            done
          '';
          postFixup = ''
            wrapProgram $out/bin/ruwa \
              --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : "${pkgs.lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins}" \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [ pkgs.libayatana-appindicator pkgs.libdbusmenu ]}"
          '';
          meta = {
            mainProgram = "ruwa";
            description = "Lightweight WhatsApp desktop client for Linux (Tauri 2 + system WebKitGTK)";
            license = nixpkgs.lib.licenses.gpl3Only;
            platforms = [ "x86_64-linux" ];
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

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/ruwa";
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            python3
            imagemagick
            nix
            pkg-config
            mold
            # Tauri 2 / WRY system webview deps
            webkitgtk_4_1
            gtk3
            libsoup_3
            cairo
            gdk-pixbuf
            glib
            pango
            atk
            librsvg
            libnotify
            libXtst
            libayatana-appindicator
            libdbusmenu
            openssl
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-ugly
            gst_all_1.gst-libav
            pipewire
            libpulseaudio
            alsa-lib
            glib-networking
            # Settings app (GTK4)
            gtk4
            graphene
            gsettings-desktop-schemas
            adwaita-icon-theme
          ];
          shellHook = ''
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
            export CARGO_INCREMENTAL=1
            export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
            export XDG_DATA_DIRS="$XDG_DATA_DIRS:${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk4}/share:${pkgs.gtk3}/share:${pkgs.adwaita-icon-theme}/share"
            echo "ruwa dev shell ready. Run: cargo test"
          '';
        };
      });
    };
}
