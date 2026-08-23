If the wifi connection is disabled and there are no other network bridge/adapter, this program cannot identify itself. 

Expected behaviour - When starting up both manager and client (order does not matter) - The program should be able to establish connection while in offline mode. It shouldn't be able to peer out internet connection, but it should simply invoke the job when resources are available locally. 

Actual behaviour - The program continues to fail to send message out stating "NoPeersSubscribedToTopic" and unable to discover each other node in offline mode. Both manager and client fail to discover each other, despite listening on correct address and port. (No loopback?)

Thoughts:
If we want to run manager and client on the same machine then ideally we'd use manager to invoke the client via cli ways. 