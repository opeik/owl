# owl

`owl` integrates Windows with the [PulseEight USB CEC adapter][cec-adapter] via [libcec][libcec].

## Features

- Hardware volume control
  - Improves audio quality by preventing Windows reducing audio bit depth
- Quick focus
    - Pressing any key will switch the HDMI source to the PC

## Setup

### Hardware

- Acquire a [HDMI-CEC adapter](cec-adpater)
- Connect the TV to the HDMI input of the adapter
- Connect the PC to the HDMI output of the adapter
- Connect the adapter to the PC via USB cable

### Software

```sh
cargo install --git https://github.com/opeik/owl.git
owl
```

[cec-adapter]: https://www.pulse-eight.com/p/104/usb-hdmi-cec-adapter
[libcec]: https://github.com/Pulse-Eight/libcec
