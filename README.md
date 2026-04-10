# ddctui

A TUI for controlling monitor settings via DDC/CI, powered by [ddcutil](https://www.ddcutil.com/).

![License](https://img.shields.io/github/license/UMCEKO/ddctui)

## Why?

The joystick button on my Samsung Odyssey G81SF disintegrated into pieces during normal use — a [known](https://us.community.samsung.com/t5/Monitors-and-Memory/Samsung-Odyssey-G9-JOG-Button-Crumbled-Not-covered-by-Warranty/td-p/3006068) [issue](https://us.community.samsung.com/t5/Monitors-and-Memory/Odyssey-G9-Jog-Button-crumbled-in-17-Days-of-purchase-No-Return/td-p/3503669) across Samsung Odyssey monitors. Without the button, there's no way to access the OSD menu.

The existing GUI tool [ddcui](https://github.com/rockowitz/ddcui) relies on parsing the monitor's DDC/CI capabilities string, which Samsung ships malformed on the G81SF. Result: ddcui shows zero controls for the monitor despite DDC/CI working fine at the protocol level.

ddctui solves both problems by probing VCP codes directly when capabilities parsing fails, and giving you a simple terminal interface to control your monitors.

## Features

- Auto-detects all monitors via ddcutil
- Supports continuous controls (brightness, contrast, RGB gains, volume, etc.)
- Supports non-continuous controls (input source, color preset, power mode, OSD language, etc.)
- Falls back to direct VCP probing when capabilities strings are broken
- 500ms debounce to avoid flooding the I2C bus
- Scrollable control list

## Install

### AUR (Arch Linux)

```
yay -S ddctui
```

### From source

```
cargo install --path .
```

Requires [ddcutil](https://www.ddcutil.com/) to be installed.

## Usage

```
ddctui
```

### Controls

| Key | Action |
|---|---|
| `↑`/`↓` or `j`/`k` | Select control |
| `←`/`→` or `h`/`l` | Adjust value |
| `+`/`-` | Adjust by 5 |
| `Tab`/`Shift+Tab` | Switch monitor |
| `q`/`Esc` | Quit |

## License

MIT
