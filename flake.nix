{
  description = "Dioxus Android development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      rust = pkgs.rust-bin.stable.latest.default.override {
        targets = [
          "aarch64-linux-android"
          "armv7-linux-androideabi"
          "i686-linux-android"
          "x86_64-linux-android"
        ];
      };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rust
          android-tools
        ];

        shellHook = ''
          export ANDROID_HOME="$HOME/Android/Sdk"
          export ANDROID_SDK_ROOT="$ANDROID_HOME"
          export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16138531"
          export PATH="$PATH:$ANDROID_HOME/emulator"
          export PATH="$PATH:$ANDROID_HOME/platform-tools"
          export PATH="$PATH:$ANDROID_HOME/cmdline-tools/latest/bin"
        '';
      };
    };
}
