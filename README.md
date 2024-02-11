# overmask

Add a writeable overlay on top of read-only files

## Installation

### Nix

```sh
$ nix run github:ErrorNoInternet/overmask -- --help
```

### cargo

```sh
$ git clone https://github.com/ErrorNoInternet/overmask
$ cd overmask
$ cargo install --path .
```

## Usage

```sh
# required for the virtual block device (/dev/nbd*)
$ sudo modprobe nbd

# create (empty) files to store data in
$ touch overlay_file mask_file

# device mode:
# read from /dev/sda, but redirect all writes to overlay_file and
# use mask_file to keep track of what has been written so that 
# future reads would read from overlay_file instead of /dev/sda
$ overmask -s /dev/sda -o overlay_file -m mask_file dev

# you can now run arbitrary write commands on the virtual block device
$ sudo dd if=/dev/zero of=/dev/nbd0
# and all the zeros would be in overlay_file instead of /dev/sda
```
