# Manyhands Initial Scaffold Design

## Goal

Create a minimal native desktop application that verifies the Rust, Devenv, GPUI, and GPUI Kit toolchain on Linux.

## Structure

The repository root will be a single Cargo binary package named `manyhands`:

```text
Cargo.toml
Cargo.lock
src/
  main.rs
devenv.nix
```

No workspace or application subcrates are needed until there is reusable application logic.

## Dependencies

The application will depend on `gpui-kit = "0.6"` with its default features. GPUI Kit manages and re-exports a compatible GPUI version, so `gpui` will not be listed as a separate direct dependency.

The Devenv configuration will provide the Linux native build and runtime dependencies needed by GPUI Kit, including compiler, font, Wayland, X11/XKB, Vulkan, and package-discovery libraries.

## Application

`src/main.rs` will initialize GPUI Kit, open one native window, and wrap a `HelloWorld` view in the GPUI Kit `Root`. The view will render centered greeting text and a primary button whose click writes a confirmation to standard output.

## Verification

Run the project through Devenv:

```sh
devenv shell -- cargo check
devenv shell -- cargo run
```

The check must compile and resolve dependencies successfully. The run must open the native window when a desktop display is available; a missing display will be documented separately rather than treated as a build failure.
