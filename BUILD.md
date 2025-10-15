 ./build-ebpf.sh && cargo install --path . && sudo setcap cap_bpf,cap_net_admin+ep /home/cyril/.local/state/cargo/bin/jail-ai
