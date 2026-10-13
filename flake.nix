{
  description = "ruwa - lightweight WhatsApp desktop client for Linux (Electron + bundled Chromium)";

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
          electron = pkgs.electron;
        in
        pkgs.stdenv.mkDerivation {
          pname = "ruwa";
          version = "0.5.0";
          src = nixpkgs.lib.cleanSource ./.;
          nativeBuildInputs = with pkgs; [
            makeWrapper
            imagemagick
          ];
          installPhase = ''
            runHook preInstall
            mkdir -p $out/share/ruwa/app $out/share/ruwa/mods $out/share/applications
            cp $src/package.json $src/main.js $src/preload-panel.js \
               $src/panel.html $src/panel.css $src/panel.js \
               $out/share/ruwa/app/
            cp $src/assets/base.css $out/share/ruwa/app/base.css
            cp $src/data/icons/ruwa.png $out/share/ruwa/app/icon.png
            cp -r $src/mods/* $out/share/ruwa/mods/
            cp $src/data/ruwa.desktop $out/share/applications/ruwa.desktop
            for size in 16 24 32 48 64 128 256 512; do
              mkdir -p $out/share/icons/hicolor/''${size}x''${size}/apps
              ${pkgs.imagemagick}/bin/magick ${./data/icons/ruwa.png} -resize ''${size}x''${size} $out/share/icons/hicolor/''${size}x''${size}/apps/ruwa.png
            done
            runHook postInstall
          '';
          postFixup = ''
            makeWrapper ${pkgs.lib.getExe electron} $out/bin/ruwa \
              --add-flags $out/share/ruwa/app \
              --set-default ELECTRON_FORCE_IS_PACKAGED 1
          '';
          meta = {
            mainProgram = "ruwa";
            description = "Lightweight WhatsApp desktop client for Linux (Electron + bundled Chromium)";
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
            nodejs
            electron
            imagemagick
            nix
          ];
          shellHook = ''
            echo "ruwa dev shell ready. Run: electron . (or: node --check main.js)"
          '';
        };
      });
    };
}
