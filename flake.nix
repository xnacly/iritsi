{
  description = "Dioxus development environment";

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
      devShells.${system} = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rust
            dioxus-cli
            android-tools
            jdk21
          ];

          shellHook = ''
            export JAVA_HOME="${pkgs.jdk21}"
            export ANDROID_HOME="$HOME/Android/Sdk"
            export ANDROID_SDK_ROOT="$ANDROID_HOME"
            export ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* | sort -V | tail -n 1)"
            export PATH="$PATH:$ANDROID_HOME/emulator"
            export PATH="$PATH:$ANDROID_HOME/platform-tools"
            export PATH="$PATH:$ANDROID_HOME/cmdline-tools/latest/bin"
          '';
        };

        desktop = pkgs.mkShell {
          packages = with pkgs; [
            rust
            dioxus-cli
            pkg-config
            webkitgtk_4_1
            gtk3
            libsoup_3
            glib
            xdotool
            openssl
          ];

          shellHook = ''
            export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
            export __EGL_VENDOR_LIBRARY_DIRS="${pkgs.mesa}/share/glvnd/egl_vendor.d"
            export LIBGL_DRIVERS_PATH="${pkgs.mesa}/lib/dri"
            export GBM_BACKENDS_PATH="${pkgs.mesa}/lib/gbm"
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
          '';
        };
      };
    };
}
