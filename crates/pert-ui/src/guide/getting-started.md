pert-ui is an unreleased desktop GUI library. Applications use `pert_ui` for
composition, bindings and native startup; no renderer, event loop or repaint
timer needs to be supplied. Native execution has been exercised on Linux
Wayland/Vulkan. Windows and macOS are targets with native verification still
outstanding. Platform accessibility adapters and plugin hosting remain deferred.

## Run your first window

Add the facade to your application's `Cargo.toml`:

```toml
[dependencies]
pert-ui = { version = "0.3" }
```

Before the first registry release, or to follow unreleased changes, depend on a
checkout of the repository instead, pointing to the facade inside **your**
checkout:

```toml
[dependencies]
pert-ui = { path = "../pert-ui/crates/pert-ui" }
```

pert-ui requires Rust 1.99.1 or later. Either dependency uses the default
`desktop` feature; a headless consumer adds `default-features = false`. The
package includes the sources of its examples, but `cargo run -p pert-ui
--example …`, as the guide runs them, works only in a checkout.

This complete program opens a profile form. Editing writes the local signals;
Reset changes the same bindings, so only their dependents refresh. Resize the
window to see the form change columns. The scroll area keeps controls reachable
when the window is too small for one usable column.

The program is the package's `examples/getting_started.rs`:
