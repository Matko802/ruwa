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
            version = "0.1.0";
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
            '';
          };
          fhsLibs = with pkgs; [
            gtk3
            dbus
            dbus-glib
            alsa-lib
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
            glib-networking
            gsettings-desktop-schemas
          ];
          init = pkgs.writeShellScript "ruwa-init" ''
            export GIO_MODULE_DIR=/usr/lib/gio/modules
            export GIO_EXTRA_MODULES=/usr/lib/gio/modules
            unset WAYLAND_DISPLAY
            exec ${tree}/share/ruwa/launch-ruwa.sh "$@"
          '';
        in
        pkgs.buildFHSEnv {
          name = "ruwa";
          targetPkgs = pkgs: fhsLibs;
          runScript = "${init}";
          extraInstallCommands = ''
            mkdir -p $out/share/applications
            cp ${./data/ruwa.desktop} $out/share/applications/ruwa.desktop
            for size in 16 24 32 48 64 128 256 512; do
              mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
              ${pkgs.imagemagick}/bin/magick ${./data/icons/ruwa.png} -resize ''${size}x''${size} $out/share/icons/hicolor/''${size}x''${size}/apps/ruwa.png
            done
          '';
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
          ];
          shellHook = ''
            echo "ruwa dev shell ready. Rebrand deps: python3 + magick."
          '';
        };
      });
    };
}
