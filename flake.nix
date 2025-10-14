{
  description = "Development environment for critters-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src" "rust-analyzer"];
      };

      nativeBuildInputs = with pkgs; [
        # Rust
        rustToolchain
        cargo-insta
        cargo-flamegraph
        samply

        # Node.js
        nodejs_24
        corepack_24
        pnpm

        # Build tools for native dependencies
        pkg-config

        # Moon task runner
        moon
      ];

      buildInputs = with pkgs;
        [
          openssl
          zlib
        ]
        ++ lib.optionals stdenv.isDarwin [
          apple-sdk
        ];
    in {
      devShells.default = pkgs.mkShell {
        inherit buildInputs nativeBuildInputs;

        shellHook = ''
          echo "critters-rs development environment"
          echo "Available tools:"
          echo "  - Rust: $(rustc --version)"
          echo "  - Node.js: $(node --version)"
          echo "  - pnpm: $(pnpm --version)"
          echo "  - Moon: $(moon --version)"
          echo ""
          echo "Quick start:"
          echo "  cargo build              # Build Rust library"
          echo "  cargo build --features cli  # Build CLI tool"
          echo "  pnpm build               # Build all packages"
          echo "  cargo test               # Run Rust tests"
          echo "  pnpm test                # Run all tests"
        '';

        # Environment variables
        RUST_BACKTRACE = "1";
        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
      };
    });
}
