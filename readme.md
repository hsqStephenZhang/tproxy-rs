## README


## settings before run the program


```bash
sysctl -w net.ipv4.conf.all.forwarding=1
```

```bash
export TPROXY_PORT=7893
export LOCAL_TO_TPROXY_MARK=6489
export TPROXY_TO_REMOTE_MARK=8964
export TCP_TEST_IP=1.0.0.1
export UDP_TEST_IP=1.0.0.1
```

```bash
ip rule add fwmark 1 lookup 100
ip route add local 0.0.0.0/0 dev lo table 100  

# to avoid the infinite loop
iptables -t mangle -N TPROXY_OUTPUT
iptables -t mangle -F TPROXY_OUTPUT
iptables -t mangle -A TPROXY_OUTPUT -j RETURN -m mark --mark $TPROXY_TO_REMOTE_MARK
iptables -t mangle -A TPROXY_OUTPUT -m addrtype --dst-type LOCAL -j RETURN
# to work with the iproute 
iptables -t mangle -A TPROXY_OUTPUT -p tcp --dst $TCP_TEST_IP --dport 80 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
iptables -t mangle -A TPROXY_OUTPUT -p udp --dst $UDP_TEST_IP --dport 53 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
iptables -t mangle -A OUTPUT -j TPROXY_OUTPUT

# to catch the output socket to the listening socket on port $TPROXY_PORT
iptables -t mangle -N TPROXY_PREROUTING
iptables -t mangle -F TPROXY_PREROUTING
iptables -t mangle -A TPROXY_PREROUTING -p tcp --dst $TCP_TEST_IP --dport 80 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
iptables -t mangle -A TPROXY_PREROUTING -p udp --dst $UDP_TEST_IP --dport 53 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
iptables -t mangle -A PREROUTING -j TPROXY_PREROUTING
```

## run program

`cargo run --tproxy-port=$TPROXY_PORT --tproxy-remote-mark=$TPROXY_TO_REMOTE_MARK`

## clean up 

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

