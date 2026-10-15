pert-ui is an unreleased desktop GUI library. Applications use `pert_ui` for
composition, bindings and native startup; no renderer, event loop or repaint
timer needs to be supplied. Native execution has been exercised on Linux
Wayland/Vulkan. Windows and macOS are targets with native verification still
outstanding. Platform accessibility adapters and plugin hosting remain deferred.

## Run your first window

Until a registry release exists, use a local checkout. In your application's
`Cargo.toml`, point to the facade inside **your** checkout:

```toml
[dependencies]
pert-ui = { path = "../pert-ui/crates/pert-ui" }
```

The workspace pins its supported Rust toolchain in `rust-toolchain.toml`; use
that toolchain when trying the checkout. This dependency uses the default
`desktop` feature. A headless consumer can add `default-features = false`.

This complete program opens a profile form. Editing writes the local signals;
Reset changes the same bindings, so only their dependents refresh. Resize the
window to see the form change columns. The scroll area keeps controls reachable
when the window is too small for one usable column.

The program is the package's `examples/getting_started.rs`:
