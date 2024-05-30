## README


## settings before run the program


```bash
sysctl -w net.ipv4.conf.all.forwarding=1
```


```bash
ip rule add fwmark 1 lookup 100
ip route add local 0.0.0.0/0 dev lo table 100  

# to avoid the infinite loop
iptables -t mangle -N TPROXY_OUTPUT
iptables -t mangle -F TPROXY_OUTPUT
iptables -t mangle -A TPROXY_OUTPUT -j RETURN -m mark --mark 0xff
iptables -t mangle -A TPROXY_OUTPUT -m addrtype --dst-type LOCAL -j RETURN
# to work with the iproute 
iptables -t mangle -A TPROXY_OUTPUT -p tcp --dst 146.190.81.132 --dport 80 -j MARK --set-mark 0x1
iptables -t mangle -A TPROXY_OUTPUT -p udp --dst 1.0.0.1 --dport 53 -j MARK --set-mark 0x1
iptables -t mangle -A OUTPUT -j TPROXY_OUTPUT

# to catch the output socket to the listening socket on port 7893
iptables -t mangle -N TPROXY_PREROUTING
iptables -t mangle -F TPROXY_PREROUTING
iptables -t mangle -A TPROXY_PREROUTING -p tcp --dst 146.190.81.132 --dport 80 -j TPROXY --tproxy-mark 0x1/0x1 --on-port 7893
iptables -t mangle -A TPROXY_PREROUTING -p udp --dst 1.0.0.1 --dport 53 -j TPROXY --tproxy-mark 0x1/0x1 --on-port 7893
iptables -t mangle -A PREROUTING -j TPROXY_PREROUTING

# todo: dns output chain

```

```bash
ip rule del fwmark 1 lookup 100
ip route flush table 100

iptables -t mangle -D OUTPUT -j TPROXY_OUTPUT
iptables -t mangle -F TPROXY_OUTPUT
iptables -t mangle -X TPROXY_OUTPUT

iptables -t mangle -D PREROUTING -j TPROXY_PREROUTING
iptables -t mangle -F TPROXY_PREROUTING
iptables -t mangle -X TPROXY_PREROUTING
```

