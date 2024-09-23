# owl

`owl` integrates Windows with the [PulseEight USB CEC adapter][cec-adapter] via [libcec][libcec].

## Features

- Control your HDMI-CEC enabled sound system via Windows
- Improved audio quality
  - Using hardware mixing prevents Windows reducing the audio bit-depth
- Quick source switch
    - Pressing any key will switch the active source to the PC

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

# License

This project is licensed under GPLv2. The [`cec`](./cec) and [`cec_sys`](./cec_sys) crates are modified versions of
[cec-rs] and [libcec-sys], respectively, and maintains their original GPLv2 license.

[cec-rs]: https://github.com/ssalonen/cec-rs
[libcec-sys]: https://github.com/ssalonen/libcec-sys
