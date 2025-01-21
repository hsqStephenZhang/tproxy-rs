## debug of one headaching bug in udp

tl;dr: to send the target's response payload back to the proxied program, from kernel's perspective, is to deliver the skb to the corresponding sk, which means the `udp_lib_lookup` or `inet_lookup` **MUST** return the recorded other end of the sk.
suppose we have one packet (192.168.2.1:34567 -> 1.1.1.1:53), but is hooked by the netfilter tproxy module, and delivered to our tproxy listening socket, the kernel's record of this udp **connection** is still (192.168.2.1:34567 -> 1.1.1.1:53), so when we send the response data back, the (src, sport) shall be (1.1.1.1:53). but how? transparent socket! 
transparent socket allows use to bind any address, for tcp, since it's fully connection based, it doesn't need this procedure - which is to say, when the tcpstream is accepted by the tproxy tcp listener, we can just read & write it like before. but for udp packet, to write it back, we have to make it a transparent one, we we can bind the target addr and port, so as not to get dropped by kernel 

### pwru log

```txt
0xffff88810457fa00 12  ~/tester:1983563 4026531840 0               0         0x86dd 1500  1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_send_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 0               0         0x86dd 1500  1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_local_out
0xffff88810457fa00 12  ~/tester:1983563 4026531840 0               0         0x86dd 1500  1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) __ip6_local_out
0xffff88810457fa00 12  ~/tester:1983563 4026531840 0               0         0x86dd 1500  1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) nf_hook_slow
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959            0         0x86dd 1500  1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_route_me_harder
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959            0         0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) __xfrm_decode_session
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959            0         0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) security_xfrm_decode_session
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959            0         0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) bpf_lsm_xfrm_decode_session
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959            0         0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_output
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) nf_hook_slow
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) apparmor_ip_postroute
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_finish_output
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_finish_output2
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) __dev_queue_xmit
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) netdev_core_pick_tx
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) validate_xmit_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) netif_skb_features
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_network_protocol
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_csum_hwoffload_help
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) validate_xmit_xfrm
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) dev_hard_start_xmit
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) loopback_xmit
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_clone_tx_timestamp
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) sock_wfree
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1086  [::1]:8888->[2606:4700:4700::1111]:53(udp) eth_type_trans
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) __netif_rx
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) netif_rx_internal
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) enqueue_to_backlog
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) __netif_receive_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) __netif_receive_skb_one_core
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ipv6_rcv
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_rcv_core
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) nf_hook_slow
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_input
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) nf_hook_slow
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_input_finish
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1072  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_protocol_deliver_rcu
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) raw6_local_deliver
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) udpv6_rcv
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) __udp6_lib_rcv
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) udp6_csum_init
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) udp6_unicast_rcv_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) udpv6_queue_rcv_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) udpv6_queue_rcv_one_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) sk_filter_trim_cap
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) security_sock_rcv_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) bpf_lsm_socket_sock_rcv_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) apparmor_socket_sock_rcv_skb
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1032  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_pull_rcsum
0xffff88810457fa00 12  ~/tester:1983563 4026531840 1959           lo:1       0x86dd 65536 1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) __udp_enqueue_schedule_skb
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_datagram_recv_common_ctl
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) ip6_datagram_recv_specific_ctl
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_consume_udp
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) __consume_stateless_skb
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_release_data
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) skb_free_head
0xffff88810457fa00 3   ~xy-demo:1982809 0          1959            0         0x86dd 0     1024  [::1]:8888->[2606:4700:4700::1111]:53(udp) kfree_skbmem
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304            0         0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_send_skb
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304            0         0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_local_out
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304            0         0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                __ip6_local_out
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304            0         0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                nf_hook_slow
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304            0         0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_output
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                nf_hook_slow
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                apparmor_ip_postroute
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_finish_output
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_finish_output2
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                __dev_queue_xmit
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                netdev_core_pick_tx
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                validate_xmit_skb
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                netif_skb_features
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                skb_network_protocol
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                skb_csum_hwoffload_help
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                validate_xmit_xfrm
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                dev_hard_start_xmit
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                loopback_xmit
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                skb_clone_tx_timestamp
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                sock_wfree
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1086  [::1]:7893->[::1]:8888(udp)                eth_type_trans
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                __netif_rx
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                netif_rx_internal
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                enqueue_to_backlog
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                __netif_receive_skb
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                __netif_receive_skb_one_core
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ipv6_rcv
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_rcv_core
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                nf_hook_slow
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_input
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                nf_hook_slow
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_input_finish
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1072  [::1]:7893->[::1]:8888(udp)                ip6_protocol_deliver_rcu
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                raw6_local_deliver
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                udpv6_rcv
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                __udp6_lib_rcv
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                udp6_csum_init
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                icmp6_send
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                security_skb_classify_flow
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                bpf_lsm_xfrm_decode_session
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                icmpv6_route_lookup
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                __xfrm_decode_session
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                security_xfrm_decode_session
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                bpf_lsm_xfrm_decode_session
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                sk_skb_reason_drop
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                skb_release_head_state
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                skb_release_data
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                skb_free_head
0xffff88816d815f00 3   ~xy-demo:1982809 4026531840 2304           lo:1       0x86dd 65536 1032  [::1]:7893->[::1]:8888(udp)                kfree_skbmem
```

when the transparent flag is not set for the **write back** udp socket, or we send data via the tproxy udp listening socket, the kernel will report `NO_SK` and drop it in `sk_skb_reason_drop` (thanks to pwru, we can locate the problem with lightning speed)
(`the udp client is a simple one, with (src, sport) bind to (::1, 8888)`)

### the struggle

at first, i have no idea why the kernel will drop the skb, so i tried to leverage bpftrace and bcc to print the drop reason, until i found this [article](https://www.manjusaka.blog/posts/2024/05/11/where-are-my-package/), then i realize that, even `pwru` didn't print out the skb drop reason like it did in this article, the problem is very much alike: the udp packet is struggling find the way home!

to verify the guess, we could check the conntrack table (the ip6tables rules for test only allows tproxy for certain target address, so the few conntrack items are just what we want to see - no other disturbance

```bash
# cat /proc/net/nf_conntrack | rg ipv6
ipv6     10 udp      17 26 src=0000:0000:0000:0000:0000:0000:0000:0001 dst=2606:4700:4700:0000:0000:0000:0000:1111 sport=8888 dport=53 [UNREPLIED] src=2606:4700:4700:0000:0000:0000:0000:1111 dst=0000:0000:0000:0000:0000:0000:0000:0001 sport=53 dport=8888 mark=0 zone=0 use=2
ipv6     10 udp      17 26 src=0000:0000:0000:0000:0000:0000:0000:0001 dst=0000:0000:0000:0000:0000:0000:0000:0001 sport=7893 dport=8888 [UNREPLIED] src=0000:0000:0000:0000:0000:0000:0000:0001 dst=0000:0000:0000:0000:0000:0000:0000:0001 sport=8888 dport=7893 mark=0 zone=0 use=2
```

from the result, we could make one simple observation: were the second item's (src, sport) changed to the first item's (dst, dport), the reply could be finished smoothly. (in this case, we didn't create a separate udp socket to write the data back, rather, we used the proxy listening socket. pretty stupid, huh?)