# Appearance Exporter (Optional)

`tiss-greetd-appearance` generates `appearance.json` for themes to consume.

Example:

```bash
tiss-greetd-appearance --wallpaper /path/to/wall.jpg --scale 1.0 --radius 18
```

It also accepts `TISS_GREETD_WALLPAPER` if `--wallpaper` is not provided.

The output path defaults to:

```
~/.local/state/tiss-greetd/appearance.json
```

For continuous updates (polling):

```bash
tiss-greetd-appearance --wallpaper-file /path/to/wallpaper.txt --watch --interval 2
```

Notes:
- The greeter reads `appearance.json` on startup.
- Live UI reload requires theme-side file watching if desired.

