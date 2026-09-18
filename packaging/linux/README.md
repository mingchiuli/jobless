# Running on Linux

Extract the tar archive and run `./jobless`.

The portable build targets Ubuntu 24.04-compatible distributions. Required
system packages include Fontconfig, X11/Wayland, Vulkan, xkbcommon, OpenSSL,
zstd and the Chromium runtime libraries. For a minimal Ubuntu installation:

```sh
sudo apt install libfontconfig1 libwayland-client0 libxkbcommon-x11-0 \
  libx11-xcb1 libssl3 libzstd1 libvulkan1
```

For CI smoke tests, install `xvfb`, `mesa-vulkan-drivers` and `libgl1-mesa-dri`
to provide an X11 display and software Vulkan/OpenGL.
