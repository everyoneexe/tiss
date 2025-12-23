# Why Not SDDM

TISS is intentionally not an SDDM clone.

## Design Goals

- Wayland-first greetd greeter with a strict QML boundary.
- Theme-agnostic core: auth/session logic never leaks into QML.
- Stable, typed IPC contract for UI themes.
- Minimal launcher that sets up seat, env, and starts cage.

## What We Avoid

- X11 legacy constraints and plugin-style theme hooks.
- Theme logic inside the display manager.
- UI frameworks tied to a single compositor ecosystem.

## Result

You get a modern lockscreen-style UX without inheriting SDDM complexity. Themes
are free to be animated and expressive while the core stays stable and secure.

