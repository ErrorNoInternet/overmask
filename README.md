# overmask
Add a writeable overlay on top of read-only files

## Installation
```sh
git clone https://github.com/ErrorNoInternet/overmask
cd overmask
cargo install --path .
```

## Usage
```sh
# required for virtual block device (/dev/nbd*)
sudo modprobe nbd

# read from /dev/sda, but redirect all writes to overlay_file
# and use mask_file to keep track of what has been written
sudo overmask --seed /dev/sda --overlay overlay_file --mask mask_file

# you can now run
sudo dd if=/dev/zero of=/dev/nbd0
# and all the zeros would be in overlay_file instead of /dev/sda
```
