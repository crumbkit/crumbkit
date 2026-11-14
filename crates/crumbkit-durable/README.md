# crumbkit-durable

crumbkit-durable replaces a file atomically. A reader sees the old bytes or
the new bytes, and after a crash or a power loss the file holds one or the
other, never part of either. The crate depends only on the standard
library, so a tool that does not use crumbkit can use it too.

## Adding it

```toml
[dependencies]
crumbkit-durable = "0.5"
```

## Example

[`write_atomically`] takes the target, the bytes and a [`Mode`]:

```rust
use crumbkit_durable::{Mode, write_atomically};

let dir = std::env::temp_dir().join(format!("crumbkit-durable-{}", std::process::id()));
std::fs::create_dir_all(&dir).unwrap();
let config = dir.join("config.toml");

write_atomically(&config, b"theme = \"dark\"\n", Mode::Umask).unwrap();
assert_eq!(std::fs::read(&config).unwrap(), b"theme = \"dark\"\n");
std::fs::remove_dir_all(&dir).unwrap();
```

`Mode::Private` gives the file `0600` on Unix, for a file such as a key, and
`Mode::Umask` gives it `0666` less the process's umask, as `File::create`
does.

## What it promises

[`write_atomically`] writes the bytes to a new temporary file beside the
target, syncs that file to disk, renames it over the target, and on Unix
syncs the target's directory, so the rename survives a power loss. The
temporary file is `.<name>.<pid>.<n>.tmp`. A name that is already taken,
such as one that a crash left behind, is skipped and its file left alone,
so two threads of one process can replace one file at once.

A symbolic link at the target is replaced by a regular file and never
followed. The new file gets the permission bits of its `Mode`, never the
old file's. The function creates no directory.

A failure returns a [`WriteError`] whose `step` names the step that failed.
A failure before the directory sync leaves the target unchanged and removes
the temporary file. A failed directory sync means the target already holds
the new bytes, and after a power loss it may hold the old ones. A directory
sync that fails with `EINVAL`, `ENOTSUP` or `ENOSYS` counts as done, because
some SMB and FUSE mounts cannot sync a directory.

## Platforms

Linux is tested on every pull request, macOS on each push to `main`, and
Windows each night.
On Windows the `Mode` is ignored, so the file inherits its directory's
access list, and the directory is not synced, because NTFS journals the
rename. Other Unix systems build but are untested, and network file systems
are out of scope.

The [durable files docs] describe the temporary file's lifecycle, and
[ADR-0221] records why the crate exists.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`write_atomically`]: src/write.rs
[`Mode`]: src/write.rs
[`WriteError`]: src/write.rs
[ADR-0221]: ../../docs/decisions/api/effects/0221-the-durable-crate.md
[durable files docs]: ../../docs/behaviour/durable-files.md
