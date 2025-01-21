## README


## settings before run the program


```bash
sysctl -w net.ipv4.conf.all.forwarding=1
```

## run

```bash
# Setup IPv4 rules with default settings
sudo ./tproxy-test.sh setup

# Setup IPv6 rules
sudo ./tproxy-test.sh --ipv6 setup

# Run the program (IPv4) or sudo cargo run --
sudo ./tproxy-test.sh run

# Run the program (IPv6) or sudo cargo run -- --ipv6
sudo ./tproxy-test.sh --ipv6 run 

# Clean up IPv4 rules
sudo ./tproxy-test.sh cleanup

# Clean up IPv6 rules
sudo ./tproxy-test.sh --ipv6 cleanup
```

## TODOs

[] optimize iptables rules
[] support udp