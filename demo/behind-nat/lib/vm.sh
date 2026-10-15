# The provider VM: Debian 13 under QEMU/KVM with user-mode networking.
#
# User-mode networking (slirp) is a NAT: the VM can open connections out, and
# nothing outside can open one in. There is no hostfwd, so no port on the host
# or the router leads to the VM. The demo drives the VM through the QEMU guest
# agent on a unix socket instead of SSH.

vm_create() {
    [ -e "$STATE/vm.pid" ] && kill -0 "$(cat "$STATE/vm.pid")" 2>/dev/null && { echo "the VM is already running"; return; }
    command -v qemu-system-x86_64 >/dev/null || die "needs qemu-system-x86_64 (pacman -S qemu-base)"
    [ -w /dev/kvm ] || die "needs read-write /dev/kvm"

    local base=$STATE/debian-13-genericcloud-amd64.qcow2
    [ -s "$base" ] || curl -fL --progress-bar -o "$base" "$VM_IMAGE_URL"

    rm -f "$STATE/disk.qcow2"
    qemu-img create -q -f qcow2 -F qcow2 -b "$base" "$STATE/disk.qcow2" "$VM_DISK"

    cat > "$STATE/meta-data" <<META
instance-id: toon-demo-provider-$(date +%s)
local-hostname: provider
META
    cat > "$STATE/user-data" <<'USER'
#cloud-config
users:
  - name: toon
    uid: 1000
    shell: /bin/bash
    lock_passwd: true
write_files:
  - path: /etc/profile.d/toon.sh
    content: |
      export PATH=$HOME/.local/bin:$PATH
      export TOON_PASSPHRASE_FILE=$HOME/.config/toon/passphrase
  - path: /etc/systemd/resolved.conf.d/no-llmnr.conf
    content: |
      [Resolve]
      LLMNR=no
      MulticastDNS=no
package_update: true
packages: [qemu-guest-agent, docker.io, curl, jq, ca-certificates, iproute2, unzip]
runcmd:
  # Nothing in this VM should listen for the outside: drop SSH.
  - [systemctl, disable, --now, ssh.service, ssh.socket]
  - [systemctl, restart, systemd-resolved]
  - [usermod, -aG, docker, toon]
  - [loginctl, enable-linger, toon]
  # The passphrase that seals the toon keystore is made here and never leaves the VM.
  - [runuser, -u, toon, --, sh, -c, "umask 077; mkdir -p /home/toon/.config/toon; head -c 32 /dev/urandom | base64 > /home/toon/.config/toon/passphrase"]
  - [systemctl, enable, --now, qemu-guest-agent]
USER
    uv run -q --with pycdlib==1.14.0 python "$DEMO_DIR/lib/mkseed.py" \
        "$STATE/seed.iso" "$STATE/user-data" "$STATE/meta-data"

    rm -f "$QGA_SOCK"
    qemu-system-x86_64 -name toon-demo-provider -machine q35,accel=kvm -cpu host \
        -smp "$VM_CPUS" -m "$VM_MEMORY" \
        -drive file="$STATE/disk.qcow2",if=virtio \
        -drive file="$STATE/seed.iso",media=cdrom,readonly=on \
        -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
        -chardev socket,path="$QGA_SOCK",server=on,wait=off,id=qga0 \
        -device virtio-serial -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0 \
        -display none -serial file:"$STATE/console.log" \
        -daemonize -pidfile "$STATE/vm.pid"
    echo "VM started, pid $(cat "$STATE/vm.pid")"
}

vm_wait() {
    local i
    for i in $(seq 1 180); do
        if python3 "$DEMO_DIR/lib/qga.py" "$QGA_SOCK" ping 2>/dev/null &&
            vm_root 'cloud-init status --wait >/dev/null 2>&1; test -S /var/run/docker.sock' >/dev/null 2>&1; then
            echo "VM ready after ~$((i * 5))s"; return
        fi
        sleep 5
    done
    die "the VM did not come up; see $STATE/console.log"
}

vm_destroy() {
    if [ -s "$STATE/vm.pid" ] && kill -0 "$(cat "$STATE/vm.pid")" 2>/dev/null; then
        kill "$(cat "$STATE/vm.pid")" && echo "VM stopped"
    fi
    rm -f "$STATE/vm.pid" "$QGA_SOCK" "$STATE/disk.qcow2" "$STATE/seed.iso"
}
