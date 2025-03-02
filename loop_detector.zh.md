## TProxy/TUN 环路检测器

### 背景

对于 TProxy 和 TUN 模式，正确配置路由表、IP 规则、iptables 规则或绑定设备至关重要，以避免路由环路。然而，有几种原因是我们无法控制的：

1. ip rule 配置错误
2. ip route 配置错误
3. 网络环境发生改变（例如 ip rule 改变, ip route 改变，设备无法绑定，设备地址改变 等等） （可以去 mihomo issue 中搜索 "tun"，即可发现一系列类似问题）

对于使用者来说，“发生环路下，将系统资源耗尽”，肯定属于 undefined behavior，代理软件预期的行为应当是：尽量避免环路，如果存在环路，则拒绝代理。

但是目前在 windows, macos, linux 等桌面系统上， mihomo, sing-tun, clash-rs 对此并无好的解决方案，主要原因是**无法判定何时发生了环路**

### TProxy 正常代理流程

以 TProxy，`curl 1.1.1.1`，代理方式为直连为例，典型的交互流程是：

1. 调用 `curl 1.1.1.1` 

2. 由于 iptables 和 IP 规则，握手数据包被定向到环回设备并交给 TProxy 监听套接字。

3. TProxy 监听套接字 (TSocket) 将响应 TCP 握手，在内核中建立连接，这样代理程序就可以使用 TSocket.accept() Hook 入站套接字。如果 curl 命令是 `curl 1.1.1.1`，那么从 TSocket 看来，连接建立成功之后，四元组将是：

远端地址：127.0.0.1:45762
本地地址：1.1.1.1:80

（三次握手时，第一个 SYN 和第三个 ACK 数据包将由 iptables 规则和 IP 规则处理，但由于第二个 SYN+ACK 数据包发送到本地 curl 程序，因此该规则对其无效。）

4. 我们（代理程序）获得真实目标服务器的地址为 1.1.1.1:80，现在可以连接此目标。将我们得到的连接称为 ingress stream， TSocket 直连到真实目标服务器的连接称为 egress stream。从代理程序的角度来看，这两个流将如下所示：

ingress stream: 127.0.0.1:45762 => 1.1.1.1:80
egress stream: 192.168.2.2:1111 => 1.1.1.1:80

5. 调用 `tokio::io::copy_bidrectional` ，双向拷贝数据，完成简单的数据转发功能。

### TProxy 环路分析

当发生环路时，第 4 步会给我们带来麻烦：当我们（代理程序）直连真实目标服务器时，egress stream 也会被 TSocket/Tun 代理，然后对于这个新的 ingress stream，则会重复前 3 个步骤，然后第 4 步的相同问题会再次发生，如此反复。

因此，发生环路时，在代理程序中，将看到一系列连接：
- 127.0.0.1:45762 => 1.1.1.1:80
- 192.168.2.2:1111 => 1.1.1.1:80
- 192.168.2.2:1112 => 1.1.1.1:80
- 192.168.2.2:1113 => 1.1.1.1:80
- ...

而这些连接，都是真实存在的，只是因由于代理程序无法正常建立 egress stream，因而导致无限递归下去，直到将系统资源耗尽。

而每次 egress stream 建立后，在代理程序看来，就已经可以接着完成上述第5步的双向拷贝了（事实上也是可以的，只不过没有任何意义），因此，会有如下的**双向拷贝连接对**:

- 127.0.0.1:45762 => 1.1.1.1:80 <=> 192.168.2.2:1111 => 1.1.1.1:80
- 192.168.2.2:1111 => 1.1.1.1:80 <=> 192.168.2.2:1112 => 1.1.1.1:80
- 192.168.2.2:1112 => 1.1.1.1:80 <=> 192.168.2.2:1113 => 1.1.1.1:80
- 192.168.2.2:1113 => 1.1.1.1:80 <=> 192.168.2.2:1114 => 1.1.1.1:80
- ...

这显然是错误的，那么如何检测呢？

### 解决思路

正如你所看到的，从第二个流开始，每个 ingress 的本地地址（连接发起者的地址）都是前一个 egress 的本地地址。我们可以利用这一点，如果在 ingress 和 egress 连接中，
都**检测**到了相同的本地地址，即可判定为**环路**
