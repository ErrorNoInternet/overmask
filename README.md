# overmask
Add a writeable overlay on top of read-only files

## Compiling
```shell
git clone https://github.com/ErrorNoInternet/overmask
cd overmask
cargo install --path .
```

### Nix
```
nix run github:ErrorNoInternet/overmask -- --help
```

## Usage
```sh
# required for virtual block device (/dev/nbd*)
sudo modprobe nbd

# read from /dev/sda, but redirect all writes to overlay_file
# and use mask_file to keep track of what has been written
touch overlay_file mask_file
sudo overmask -s /dev/sda -o overlay_file -m mask_file

# you can now run
sudo dd if=/dev/zero of=/dev/nbd0
# and all the zeros would be in overlay_file instead of /dev/sda
```
