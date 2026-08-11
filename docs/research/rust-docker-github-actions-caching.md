# Rust and Docker caching for GitHub Actions

Date: 2026-08-11

## Conclusion

The current workflow already uses the standard Docker pattern: `cargo-chef`,
BuildKit cache mounts, `buildkit-cache-dance`, and scoped Docker GitHub Actions
caches. Replacing `type=gha` or `mode=max` is unlikely to improve this job.
Docker documents the GitHub Actions backend as the recommended backend for
GitHub Actions and recommends separate scopes when more than one image uses the
cache:

- <https://docs.docker.com/build/cache/backends/gha/>
- <https://github.com/LukeMathWalker/cargo-chef>
- <https://github.com/reproducible-containers/buildkit-cache-dance>

The largest remaining opportunity is the 4.6 GB Rust cache observed in recent
smoke runs. `buildkit-cache-dance` must copy the cached `target/` directory into
the BuildKit builder before a build can use it. A full target cache can therefore
cost more time to restore and inject than a cache hit saves.

## Patterns found in other repositories

### 1. Keep Cargo source caches, replace `target/` with sccache

vLLM uses a Rust build stage that caches only Cargo’s registry and Git
directories. It explicitly avoids caching `target/` because stale target
metadata can survive source changes. Compiler outputs are handled by `sccache`
instead:

- <https://github.com/vllm-project/vllm/blob/main/docker/Dockerfile#L244-L296>
- <https://github.com/mozilla-actions/sccache-action/blob/main/README.md#rust-code>

This is the best candidate if dcapal’s source changes frequently. It removes
the multi-gigabyte target injection and still reuses compiled Rust objects when
the compiler inputs match. It is more complex than the current setup because
the sccache configuration and credentials must be passed safely into the
BuildKit build; the GitHub Actions environment is not automatically available
inside a Docker build.

### 2. Use a release profile without debug information for smoke images

Cargo documents that the normal `release` profile has no debug information,
while dcapal’s `release-with-debug` profile enables full debug information.
Debug information is included in compiled artifacts and can make a target cache
much larger:

- <https://doc.rust-lang.org/cargo/reference/profiles.html#debug>
- <https://doc.rust-lang.org/cargo/reference/profiles.html#release>

Bevy sets `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and
`CARGO_PROFILE_DEV_DEBUG=0` for CI cache efficiency:

- <https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml>

Tauri documents the same reason for setting `CARGO_PROFILE_DEV_DEBUG=0` and
uses `Swatinem/rust-cache` with one matrix leg responsible for saving the
cache:

- <https://github.com/tauri-apps/tauri/blob/dev/.github/workflows/test-core.yml>
- <https://github.com/Swatinem/rust-cache/blob/master/action.yml>

For dcapal, build the smoke image with the normal `release` profile and keep
`release-with-debug` for publish or diagnostic builds. The profile must be part
of the Rust cache key; otherwise smoke and publish could restore incompatible
target contents.

### 3. Build Rust outside Docker, then package the artifact

Tauri builds its Rust CLI in a setup job, uploads the binary, and downloads it
in the Docker job. fuse-overlayfs uses the same shape for its build and
integration-test jobs:

- <https://github.com/tauri-apps/tauri/blob/dev/.github/workflows/docker.yml#L14-L49>
- <https://github.com/containers/fuse-overlayfs/blob/main/.github/workflows/rust-test.yaml#L5-L46>

Applied to dcapal, a host-side Rust job could use the existing Rust cache action
and upload `dcapal-backend` and `migration`. The Docker build would then only
assemble the runtime image, retaining Docker layer caching without requiring
cache-dance for Rust compilation. This can be faster, but it changes the job
shape and makes the artifact boundary part of the smoke-test contract.

### 4. Prewarm caches on a trusted branch

Bevy has a scheduled cache-building workflow. It uses `actions/cache/restore`
with `lookup-only`, builds the cache only on a miss, and saves it from a
dedicated job. Its normal CI jobs restore the prepared cache:

- <https://github.com/bevyengine/bevy/blob/main/.github/workflows/update-caches.yml>
- <https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml>

This prevents pull requests from competing to save large caches and makes cache
contents more predictable. It does not remove the download cost for a smoke job,
so it is a secondary improvement for dcapal.

## Recommended order for dcapal

1. Keep the existing Docker GHA scopes, `mode=max`, `cargo-chef`, and
   `buildkit-cache-dance`.
2. Add `rust-toolchain.toml`, `.cargo/config.toml` if present, and the selected
   Cargo profile to the Rust cache key.
3. Build smoke images with `release` rather than `release-with-debug`, using a
   separate cache prefix from publish builds. Measure cache size, cache-dance
   time, and cold-build time.
4. If the target cache remains the dominant cost, prototype the vLLM pattern:
   cache only Cargo registry/Git data and use sccache for compiler outputs.
5. If sccache setup is not worth the complexity, split Rust compilation into a
   host job and use artifact download plus a runtime-only Docker packaging job.

The first experiment is low risk and directly addresses the measured cache
size. The sccache and artifact approaches are larger design changes and should
be evaluated with cold, warm, and source-change runs before adoption.
