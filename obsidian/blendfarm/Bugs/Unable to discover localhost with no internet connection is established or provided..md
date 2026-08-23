Currently in the offline mode (Airplane mode/wifi turned off/no ethernet adapter/etc), having both manager and client will not discover itself. 

Todo - contact the community or research online on how to achieve loopback rules for the firewall? I thought it was possible to communicate through a separate channel?

Expected behavior, Both manager and client should discover itself and begin communication within 0.0.0.0 address

Actual behavior: Client and manager unable to find each other.

still an issue 5/9/2026 - Seems like there's no loopback rules to identify the machine itself - May be useful just to rely on cli commands and invoke the program itself manually.