# Bun, Vale and rustup for the workbench and the engine. `direnv allow` loads
# the shell; the commands are the package.json scripts (`bun run build`) and
# cargo. rustup takes Rust from rust-toolchain.toml.
{ pkgs, options, ... }:

{
  # No Nix compiler, Apple SDK or xcrun shim: cargo links with the host's
  # compiler, as it does under a plain rustup.
  stdenv = pkgs.stdenvNoCC;
  apple.sdk = null;
  # stdenv sets these for Nix builds; they would pin native binaries' minimum
  # macOS to the SDK's and stamp build outputs with 1980.
  unsetEnvVars = options.unsetEnvVars.default ++ [
    "SOURCE_DATE_EPOCH"
    "MACOSX_DEPLOYMENT_TARGET"
    "ZERO_AR_DATE"
  ];

  packages = [
    # package.json's packageManager names the same version for CI.
    pkgs.bun
    pkgs.rustup
    # The prose workflow downloads the same release.
    pkgs.vale
  ];
}
