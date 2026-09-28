{
  description = "Convert Markdown to a self-contained, print-ready HTML document";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;

      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      forAllSystems = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      cargoToml = lib.importTOML ./Cargo.toml;

      # Only what the build reads, so ./target and ./result never invalidate it.
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./src
        ];
      };
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = md2html;

        md2html = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          inherit (cargoToml.package) version;
          inherit src;

          cargoLock.lockFile = ./Cargo.lock;

          postInstall = ''
            $out/bin/md2html --version > /dev/null
          '';

          meta = {
            inherit (cargoToml.package) description;
            homepage = cargoToml.package.repository;
            license = lib.licenses.mit;
            mainProgram = "md2html";
            platforms = lib.platforms.unix;
          };
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
          ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      checks = forAllSystems (
        pkgs:
        let
          inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) md2html;
        in
        {
          # `cargo test` already runs in md2html's checkPhase.
          build = md2html;

          clippy = md2html.overrideAttrs (old: {
            pname = "${old.pname}-clippy";
            nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.clippy ];
            buildPhase = "cargo clippy --release --all-targets -- --deny warnings";
            doCheck = false;
            installPhase = "touch $out";
          });

          rustfmt =
            pkgs.runCommand "md2html-rustfmt" { nativeBuildInputs = [ pkgs.rustfmt ]; }
              ''
                rustfmt --check --edition ${cargoToml.package.edition} ${src}/src/*.rs
                touch $out
              '';

          # The rendered document must stand alone: styles inlined, no network.
          render = pkgs.runCommand "md2html-render" { nativeBuildInputs = [ md2html ]; } ''
            printf '# Title\n\nBody with a [link](https://example.com).\n' > doc.md
            md2html doc.md -o out.html

            grep -q '<title>Title</title>' out.html
            grep -q '@media print' out.html
            grep -q '<h1>Title</h1>' out.html
            grep -qv 'https://fonts' out.html

            md2html --fragment doc.md | grep -qv '<!DOCTYPE'
            md2html doc.md | diff - out.html
            printf '# Piped\n' | md2html - | grep -q '<h1>Piped</h1>'

            touch $out
          '';
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
