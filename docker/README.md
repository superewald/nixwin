# Nixwin Container Images

Nixwin provides container images with bundled sysroots for all supported vs manifest versions.

**base images**

Base images contain sysroots (CRT/SDK) for x86_64 platform and common compilation tools (llvm, cmake, make, ...). Nixwin provides images based on ubuntu and alpine: `nixwin:$VS_VERSION` and `nixwin:$VS_VERSION-alpine`.

- `nixwin:16[-alpine]`
- `nixwin:17[-alpine]`
- `nixwin:18[-alpine]`

**flavors**

Additionally, each base image has the following feature extensions:

- `nixwin:$VS_VERSION-debug`: adds debug libraries
- `nixwin:$VS_VERSION-wine`: adds debug libraries and wine
- `nixwin:$VS_VERSION-aarch`: adds aarch/aarch64 platform
- `nixwin:$VS_VERSION-aarch-debug`: adds aarch/aarch64 platform and debug libraries
- `nixwin:$VS_VERSION-aarch-wine`: adds aarch/aarch64 platform, debug libraries and wine

**seamless docker integration**

The nixwin image can be used locally for neat encapsulation:

```sh
docker pull nixwin:17-wine
alias nixwin="docker run -it --rm -v \"$PWD:/app\" -w /app nixwin:17-wine nixwin"
nixwin ls # VS 17
nixwin install
```