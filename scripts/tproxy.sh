#!/bin/bash

# Exit on error
set -e

# Default values
TPROXY_PORT=7893
LOCAL_TO_TPROXY_MARK=6489
TPROXY_TO_REMOTE_MARK=8964
IPV4_TCP_TEST_IP="1.0.0.1"
IPV4_UDP_TEST_IP="1.0.0.1"
IPV6_TCP_TEST_IP="2606:4700:4700::1111"
IPV6_UDP_TEST_IP="2606:4700:4700::1111"

# Function to display usage
usage() {
    echo "Usage: $0 [OPTIONS] COMMAND"
    echo
    echo "Commands:"
    echo "  setup     Set up TPROXY rules"
    echo "  run       Run the program"
    echo "  cleanup   Clean up TPROXY rules"
    echo
    echo "Options:"
    echo "  --ipv6                  Use IPv6 (default: IPv4)"
    echo "  --port PORT             TPROXY port (default: 7893)"
    echo "  --local-mark MARK       Local to TPROXY mark (default: 6489)"
    echo "  --remote-mark MARK      TPROXY to remote mark (default: 8964)"
    echo "  --tcp-ip IP             TCP test IP address"
    echo "  --udp-ip IP             UDP test IP address"
    echo "  -h, --help              Show this help message"
}

# Function to set up IPv4 rules
setup_ipv4() {
    echo "Setting up IPv4 TPROXY rules..."
    
    ip rule add fwmark $LOCAL_TO_TPROXY_MARK lookup 100
    ip route add local 0.0.0.0/0 dev lo table 100

    # Create and configure TPROXY_OUTPUT chain
    iptables -t mangle -N TPROXY_OUTPUT
    iptables -t mangle -F TPROXY_OUTPUT
    iptables -t mangle -A TPROXY_OUTPUT -j RETURN -m mark --mark $TPROXY_TO_REMOTE_MARK
    iptables -t mangle -A TPROXY_OUTPUT -m addrtype --dst-type LOCAL -j RETURN
    iptables -t mangle -A TPROXY_OUTPUT -p tcp --dst $IPV4_TCP_TEST_IP --dport 80 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
    iptables -t mangle -A TPROXY_OUTPUT -p udp --dst $IPV4_UDP_TEST_IP --dport 53 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
    iptables -t mangle -A OUTPUT -j TPROXY_OUTPUT

    # Create and configure TPROXY_PREROUTING chain
    iptables -t mangle -N TPROXY_PREROUTING
    iptables -t mangle -F TPROXY_PREROUTING
    iptables -t mangle -A TPROXY_PREROUTING -p tcp --dst $IPV4_TCP_TEST_IP --dport 80 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
    iptables -t mangle -A TPROXY_PREROUTING -p udp --dst $IPV4_UDP_TEST_IP --dport 53 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
    iptables -t mangle -A PREROUTING -j TPROXY_PREROUTING
}

# Function to set up IPv6 rules
setup_ipv6() {
    echo "Setting up IPv6 TPROXY rules..."
    
    ip -6 rule add fwmark $LOCAL_TO_TPROXY_MARK lookup 101
    ip -6 route add local ::/0 dev lo table 101

    # Create and configure TPROXY_OUTPUT chain
    ip6tables -t mangle -N TPROXY_OUTPUT
    ip6tables -t mangle -F TPROXY_OUTPUT
    ip6tables -t mangle -A TPROXY_OUTPUT -j RETURN -m mark --mark $TPROXY_TO_REMOTE_MARK
    ip6tables -t mangle -A TPROXY_OUTPUT -m addrtype --dst-type LOCAL -j RETURN
    ip6tables -t mangle -A TPROXY_OUTPUT -p tcp --dst $IPV6_TCP_TEST_IP --dport 80 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
    ip6tables -t mangle -A TPROXY_OUTPUT -p udp --dst $IPV6_UDP_TEST_IP --dport 53 -j MARK --set-mark $LOCAL_TO_TPROXY_MARK
    ip6tables -t mangle -A OUTPUT -j TPROXY_OUTPUT

    # Create and configure TPROXY_PREROUTING chain
    ip6tables -t mangle -N TPROXY_PREROUTING
    ip6tables -t mangle -F TPROXY_PREROUTING
    ip6tables -t mangle -A TPROXY_PREROUTING -p tcp --dst $IPV6_TCP_TEST_IP --dport 80 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
    ip6tables -t mangle -A TPROXY_PREROUTING -p udp --dst $IPV6_UDP_TEST_IP --dport 53 -j TPROXY --tproxy-mark $LOCAL_TO_TPROXY_MARK/$LOCAL_TO_TPROXY_MARK --on-port $TPROXY_PORT
    ip6tables -t mangle -A PREROUTING -j TPROXY_PREROUTING
}

# Function to clean up IPv4 rules
cleanup_ipv4() {
    echo "Cleaning up IPv4 TPROXY rules..."
    
    ip rule del fwmark $LOCAL_TO_TPROXY_MARK lookup 100
    ip route flush table 100
    
    iptables -t mangle -D OUTPUT -j TPROXY_OUTPUT
    iptables -t mangle -F TPROXY_OUTPUT
    iptables -t mangle -X TPROXY_OUTPUT
    iptables -t mangle -D PREROUTING -j TPROXY_PREROUTING
    iptables -t mangle -F TPROXY_PREROUTING
    iptables -t mangle -X TPROXY_PREROUTING
}

# Function to clean up IPv6 rules
cleanup_ipv6() {
    echo "Cleaning up IPv6 TPROXY rules..."
    
    ip -6 rule del fwmark $LOCAL_TO_TPROXY_MARK lookup 101
    ip -6 route flush table 101
    
    ip6tables -t mangle -D OUTPUT -j TPROXY_OUTPUT
    ip6tables -t mangle -F TPROXY_OUTPUT
    ip6tables -t mangle -X TPROXY_OUTPUT
    ip6tables -t mangle -D PREROUTING -j TPROXY_PREROUTING
    ip6tables -t mangle -F TPROXY_PREROUTING
    ip6tables -t mangle -X TPROXY_PREROUTING
}

# Parse command line arguments
USE_IPV6=0
COMMAND=""

while [[ $# -gt 0 ]]; do
    case $1 in
        setup|run|cleanup)
            COMMAND="$1"
            shift
            ;;
        --ipv6)
            USE_IPV6=1
            shift
            ;;
        --port)
            TPROXY_PORT="$2"
            shift 2
            ;;
        --local-mark)
            LOCAL_TO_TPROXY_MARK="$2"
            shift 2
            ;;
        --remote-mark)
            TPROXY_TO_REMOTE_MARK="$2"
            shift 2
            ;;
        --tcp-ip)
            if [ $USE_IPV6 -eq 1 ]; then
                IPV6_TCP_TEST_IP="$2"
            else
                IPV4_TCP_TEST_IP="$2"
            fi
            shift 2
            ;;
        --udp-ip)
            if [ $USE_IPV6 -eq 1 ]; then
                IPV6_UDP_TEST_IP="$2"
            else
                IPV4_UDP_TEST_IP="$2"
            fi
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Check if script is run as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# Check if command is provided
if [ -z "$COMMAND" ]; then
    echo "Error: Command required"
    usage
    exit 1
fi

# Execute requested command
case $COMMAND in
    setup)
        if [ $USE_IPV6 -eq 1 ]; then
            setup_ipv6
        else
            setup_ipv4
        fi
        ;;
    run)
        if [ $USE_IPV6 -eq 1 ]; then
            cargo run -- --ipv6 --tproxy-port=$TPROXY_PORT --tproxy-remote-mark=$TPROXY_TO_REMOTE_MARK
        else
            cargo run -- --tproxy-port=$TPROXY_PORT --tproxy-remote-mark=$TPROXY_TO_REMOTE_MARK
        fi
        ;;
    cleanup)
        if [ $USE_IPV6 -eq 1 ]; then
            cleanup_ipv6
        else
            cleanup_ipv4
        fi
        ;;
esac