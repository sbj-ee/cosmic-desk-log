# COSMIC Desk Log

A Conky-style desk widget for [COSMIC](https://system76.com/cosmic) that streams
the systemd journal on a Wayland **Bottom** layer surface — above the wallpaper,
under every normal window — spanning the screen at the top or bottom.

Clicks pass through the widget to the desktop beneath it.

## Requirements

- Pop!_OS / COSMIC session (Wayland)
- `journalctl` (systemd)
- Rust toolchain (to build from source)

## Install

```bash
just install
systemctl --user enable --now cosmic-desk-log.service
```

Or in one step:

```bash
just enable
```

## Uninstall

```bash
just uninstall
```

## Configuration

Optional file: `~/.config/cosmic-desk-log/config`

```
# Logical size of the desk panel
# width=full (default) → the full active display via cosmic-randr, less margins
# width=half           → ~50% of the active display
# width=960            → fixed pixels
width=full
height=280

# Top or bottom of the screen
position=bottom
# position=top

# Margins from the side + chosen vertical edge (clear of panel/dock).
# margin_left is mirrored on the right when width=full.
margin_left=24
margin_top=48
margin_bottom=72

# Appearance
font_size=12
opacity=0.35

# Keep this many lines in the ring buffer
max_lines=200

# Extra journalctl arguments (space-separated), appended after the defaults
# Defaults are: -f -o short-iso --no-pager -n <max_lines>
# journal_args=-p err..warning
# journal_args=_TRANSPORT=kernel
```

Restart the service after editing:

```bash
systemctl --user restart cosmic-desk-log.service
```

## Run without systemd

```bash
cargo run --release
```

## Notes

- Coexists with `cosmic-bg` / `cosmic-audio-bg` (those use the Background layer).
- Reading the system journal may require membership in the `systemd-journal` or
  `adm` group, depending on your distro defaults.

## License

[MIT](LICENSE)
