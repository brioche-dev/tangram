# vm

To build qemu, download an Ubuntu Server image, boot, and configure a VM:

```sh
# Grab some dependencies
$ brew install cdrtools

# Download boot image, download and build qemu, build vm cloud-init iso
# (takes a few minutes)
$ make

# Boot the VM
$ ./boot.sh

# In another terminal, connect to the serial port (requires socat)
# Log in with tangram:tangram
$ ./connect.sh guest/serial.sock

# At this point, running 'sudo ls /mnt/host_home' in the VM should list your host home directory
```
