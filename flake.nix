{
  description = "ruwa - lightweight WhatsApp desktop client for Linux (Neutron/Firefox SSB, derived from Datcord)";

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
          datcordSrc = pkgs.fetchurl {
            url = "https://github.com/gamingdoom/datcord/releases/download/0.6.2/datcord-linux-x86_64.tar.bz2";
            hash = "sha256-Hc+z09yBWjOOw5iq8Unr6dNV0ZupeMJA2LWMZmql1dg=";
          };
          tree = pkgs.stdenv.mkDerivation {
            pname = "ruwa-tree";
            version = "0.2.0";
            src = datcordSrc;
            dontConfigure = true;
            dontBuild = true;
            nativeBuildInputs = with pkgs; [
              python3
              imagemagick
            ];
            installPhase = ''
              srcdir=$(ls -d datcord-*/ | head -1)
              echo "rebranding $srcdir"
              python3 ${./neutron/rebrand.py} \
                --src "$srcdir" \
                --dst $out/share/ruwa \
                --logo ${./data/icons/ruwa.png}
              install -Dm755 ${./neutron/launch-ruwa.sh} $out/share/ruwa/launch-ruwa.sh
              install -Dm644 ${./neutron/userChrome.css} $out/share/ruwa/chrome-dist/userChrome.css
              install -Dm644 ${./neutron/userContent-base.css} $out/share/ruwa/chrome-dist/userContent-base.css
            '';
          };
          fhsLibs = with pkgs; [
            gtk3
            dbus
            dbus-glib
            alsa-lib
            libpulseaudio
            pipewire
            libxkbcommon
            cups
            libdrm
            mesa
            libxtst
            libxscrnsaver
            nss
            nspr
            expat
            libxkbfile
            glib
            pango
            cairo
            gdk-pixbuf
            atk
            at-spi2-atk
            libX11
            libxcb
            libXext
            libXi
            libXfixes
            libXrender
            libXcursor
            libXdamage
            libXcomposite
            libXrandr
            fontconfig
            freetype
            libXau
            libXdmcp
            libbsd
            libmd
            libappindicator-gtk3
            libdbusmenu
            pciutils
            systemd
            libva
            ffmpeg_7
            glib-networking
            gsettings-desktop-schemas
          ];
          init = pkgs.writeShellScript "ruwa-init" ''
            export GIO_MODULE_DIR=/usr/lib/gio/modules
            export GIO_EXTRA_MODULES=/usr/lib/gio/modules
            case "''${RUWA_BACKEND:-auto}" in
              x11|X11|xwayland) unset WAYLAND_DISPLAY ;;
              *) ;;
            esac
            exec ${tree}/share/ruwa/launch-ruwa.sh "$@"
          '';
          supervisor = pkgs.rustPlatform.buildRustPackage {
            pname = "ruwa-supervisor";
            version = "0.2.0";
            src = nixpkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = with pkgs; [
              pkg-config
              wrapGAppsHook4
              makeWrapper
            ];
            buildInputs = with pkgs; [
              gtk4
              glib
              pango
              gdk-pixbuf
              cairo
              graphene
              gsettings-desktop-schemas
              adwaita-icon-theme
            ];
            postInstall = ''
              mkdir -p $out/share/applications
              cp ${./data/ruwa.desktop} $out/share/applications/ruwa.desktop
              for size in 16 24 32 48 64 128 256 512; do
                mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
                ${pkgs.imagemagick}/bin/magick ${./data/icons/ruwa.png} -resize ''${size}x''${size} $out/share/icons/hicolor/''${size}x''${size}/apps/ruwa.png
              done
              wrapProgram $out/bin/ruwa --set-default RUWA_ENGINE ${engine}/bin/ruwa-engine
            '';
          };
          engine = pkgs.buildFHSEnv {
            name = "ruwa-engine";
            targetPkgs = pkgs: fhsLibs;
            runScript = "${init}";
            # Host /run carries the PipeWire/PulseAudio sockets and the
            # D-Bus session bus; without it there is no audio and no
            # native web notifications. dieWithParent keeps Quit reliable:
            # killing the wrapper always takes the engine down with it.
            dieWithParent = true;
            extraBwrapArgs = [ "--bind /run /run" ];
          };
        in
        pkgs.symlinkJoin {
          name = "ruwa";
          paths = [
            supervisor
            engine
          ];
          meta = {
            mainProgram = "ruwa";
            description = "Lightweight WhatsApp desktop client for Linux (Neutron/Firefox SSB)";
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
            gtk4
            glib
            pango
            gdk-pixbuf
            cairo
            graphene
            gsettings-desktop-schemas
            adwaita-icon-theme
          ];
          shellHook = ''
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
            export CARGO_INCREMENTAL=1
            export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
            export XDG_DATA_DIRS="$XDG_DATA_DIRS:${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk4}/share:${pkgs.adwaita-icon-theme}/share"
            echo "ruwa dev shell ready. Run: cargo test"
          '';
        };
      });
    };
}
