# Container images

For builds that should not touch the host at all, prebuilt `nixwin` container
images bundle a sysroot together with the compilation tools.

!!! note "The images are built outside this repository"

    This repository contains no `Dockerfile` and no image build. The published
    images are produced by separate automation, so you can neither rebuild a
    tag locally nor expect the tag list here to change when a new image is
    pushed. Pull the images from the registry; if you need a sysroot that is not
    published, build one with [`nixwin install`](cli.md#install) instead.

## Base images

Base images contain the CRT and SDK sysroot for `x86_64` plus the common
compilation tools (llvm, cmake, make, …), on both Ubuntu and Alpine:

```
nixwin:16        nixwin:16-alpine
nixwin:17        nixwin:17-alpine
nixwin:18        nixwin:18-alpine
```

`$VS_VERSION` is the Visual Studio manifest version the sysroot was resolved
from. The non-Alpine variants are Ubuntu-based; the `-alpine` ones use musl and
are smaller.

## Flavors

Each base image has feature extensions, combined by suffix:

| image | contents |
|---|---|
| `nixwin:$VS_VERSION` | sysroot for `x86_64` |
| `nixwin:$VS_VERSION-debug` | adds the debug libraries |
| `nixwin:$VS_VERSION-wine` | adds the debug libraries and wine, so binaries can be run |
| `nixwin:$VS_VERSION-aarch` | adds the `aarch`/`aarch64` platform |
| `nixwin:$VS_VERSION-aarch-debug` | `aarch`/`aarch64` plus debug libraries |
| `nixwin:$VS_VERSION-aarch-wine` | `aarch`/`aarch64` plus debug libraries and wine |

So the full set for a manifest version is `nixwin:17`, `nixwin:17-alpine`,
`nixwin:17-debug`, `nixwin:17-wine`, `nixwin:17-aarch`, `nixwin:17-aarch-debug`
and `nixwin:17-aarch-wine`, with the same suffixes on every other
`$VS_VERSION`.

## Using an image locally

Mount the working directory at a fixed path and run `nixwin` from it, so the
sysroot and the sources live in the same container:

```sh
docker pull nixwin:17-wine
alias nixwin="docker run -it --rm -v \"$PWD:/app\" -w /app nixwin:17-wine nixwin"

nixwin ls        # the VS 17 sysroot is already in the image
nixwin install   # or build one yourself inside the container
```

Because `$PWD` is mounted at `/app`, a `.nixwin.json` you generate with
`--lock` is written back to your host repository and can be committed, which is
how a CI job and your laptop end up with the same component versions.

## Using an image in CI

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    container:
      image: nixwin:17-aarch-debug
    steps:
      - uses: actions/checkout@v4
      - run: nixwin install --lock .   # reuse the committed lockfile
      - run: cmake -B build && cmake --build build
```

A lockfile in the repository makes the image mostly a source of the
compilation tools: `nixwin install` then resolves the exact versions the
lockfile pins, rather than whatever the image happened to be built with. See
[lockfiles](lockfiles.md) for how the components are combined.
