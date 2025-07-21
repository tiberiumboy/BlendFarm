Server must first create a socket object
then we bind the socket to the address, port.
afterward, we listen to the ports.
finally, we accept the listen call. 

client must create socket object, makes connection.
client will "connect" through socket. 
client must connect to the same ip:port as the server socket.

once the server accepts, we now have a connection established between client and server to freely exchange data between those two connections.

socket file descriptor that can write and read data.
