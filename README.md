# overmask

Add a writeable overlay on top of read-only files

## Installation

Nix flake: `github:ErrorNoInternet/overmask`

AUR: https://aur.archlinux.org/packages/overmask

COPR: https://copr.fedorainfracloud.org/coprs/errornointernet/packages

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
# future reads would be from overlay_file instead of /dev/sda
$ overmask -s /dev/sda -o overlay_file -m mask_file dev

# you can now send arbitrary write commands to the virtual block device
$ sudo dd if=/dev/zero of=/dev/nbd0
# and all the zeros would be in overlay_file instead of /dev/sda
```
