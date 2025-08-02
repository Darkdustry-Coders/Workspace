# Workspace

Workspace for testing and developing Mindurka.

## Using the workspace

Before doing anything, run the wrapper with `./b`.

## Defining targets

Targets are defined by creating a new file in `buildscript/src/targets/<name>.rs`
and then registering it in `buildscript/src/targets.rs`.

A target struct must be named `Impl` and implement `TargetImpl` and `TargetImplStatic`.

i.e. for target named `test`

`buildscript/src/targets/test.rs`
```rust
struct Impl {}

impl TargetImpl for Impl {
    // ..
}

impl TargetImplStatic for Impl {
    // ..
}
```

`buildscript/src/targets.rs`
```rust
// At the bottom of the file.
targets! {
    test: Test;
}
```

## Paths

This will not work on weird paths and I don't care.

## NixOS, MacOS, anything ARM, or other unsupported systems

Feel free to PR
