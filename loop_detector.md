## loop detector for tproxy/tun 

### background 

for tproxy and tun, it's crucial to configure the routing table, ip rules, iptables rules or bind the device correctly to avoid routing loop. however, there are two main causes that are beyond our control:

1. misconfiguration. the loop is likely to occur if:
    a. for tproxy, the option `iptables` is not enabled (take mihomo as example, this option will save user the trouble of configuring `ip rule` and `iptables`)
    b. for tun, if the routing rule is not setup correctly
2. network condition in the real work. 
    a. for tun, the outbound packet shall be binded to a certain device/address to bypass the routing rules, avoiding been hooked by the proxy program and stuck into the loop. but things are a little stricky if the active NICs are changing for reasons like the change of network environment. 

so, the proxy program and user would be happy if we can tell whether the looping occurs, if so, the better options for the proxy program should be dropping the connection, avoiding eating up the resources of the precious(cpu & mem)

### solution

the ideas are simple: ownership of socket. let me break this down for you step by step.

taking tproxy as example, the typical interaction should be:
1. the local curl program is called
2. thanks to the iptables and ip rule, the handshake packets are directed to loopback device and handed to the tproxy listening socket.
3. the tproxy listening socket(TSocket) will respond to the tcp handshake, build the connection in kernel, so the proxy program can hook the inbound socket with `TSocket.accept()`, if the curl command is `curl 1.1.1.1`, then the inbound socket would be:
    - remote addr: `127.0.0.1:45762`
    - local addr: `1.1.1.1:80`

    (the first syn and the third act packet will be processed by iptables rules and ip rules, but since the second syn+ack packet is sent to the local `curl` program, the rule won't work on it)
4. we get the real target server's addr, so it's time to dial this target and connection the hooked stream with the stream dialed by us. from the proxy program's view, the two streams will look like:
    - stream1: `127.0.0.1:45762 => 1.1.1.1:80`
    - stream2: `192.168.2.2:1111 => 1.1.1.1:80`
5. copy bidirectional

when looping occurs, the step4 would bring trouble for us: when dialing the remote, this stream dialed by us(proxy program) will **also** be hooked by `TSocket`, then the first 3 steps are repeated normally, then the same issue in step4 will occur again, so on and so forth. 
so, now the TSocket will see a serious of streams:
    - `127.0.0.1:45762 => 1.1.1.1:80`
    - `192.168.2.2:1111 => 1.1.1.1:80`
    - `192.168.2.2:1112 => 1.1.1.1:80`
    - `192.168.2.2:1113 => 1.1.1.1:80`
    - ...

so there would be tons of pairs of stream to copy bidirectionally:
    - `127.0.0.1:45762 => 1.1.1.1:80` <=> `192.168.2.2:1111 => 1.1.1.1:80`
    - `192.168.2.2:1111 => 1.1.1.1:80` <=> `192.168.2.2:1112 => 1.1.1.1:80`
    - `192.168.2.2:1112 => 1.1.1.1:80` <=> `192.168.2.2:1113 => 1.1.1.1:80`
    - `192.168.2.2:1113 => 1.1.1.1:80` <=> `192.168.2.2:1114 => 1.1.1.1:80`
    - ...

which is obviously wrong, so how to detect the error? 

as you can see, starting from the second stream, every inbound stream's address is the address of previous outbound stream's. we may utilize this point. Voila!

se so the rule is: **every outbound socket address that is owned by us(proxy program), should not occur in tproxy(or tun)'s inbound stage.** 

### implementation notes

the programmer may introduce a global shared hashset of socket addresses. when a new socket is created by proxy program and connected, the address of this socket is inserted to the hashset. when the socket is closed/dropped, the address of this socket is removed from the hashset. then, in the tproxy/tun inbound handler, the proxy just need to check if the address has already been in the hashset. if so, report warning or error msg.
to make it more robust, the programmer may uses a hashmap from socket address to its occurance count, in both inbound handler and after outbound socket is connected, the count is added by one, if the count equals to two, then report warning or error msg. this is to prevent the kernel from waking the client/server side in the reverse order. 