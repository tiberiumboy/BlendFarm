The front facing application used to interface all services running in the background, as well as allowing user friendly interface to create a new job to run. The methodology is that the client can run the host on any computer, upload blender files for the network to fetch, and instruct available node on the same network to process the job.

The flow of the app goes:
Upon creating a new job description, the host will broadcast signal notifying all idle clients a new job is available to process. 

When the node respond with a request new ticket, a ticket will be generate and assigned to that specific peer. This host will send the peer the ticket description over the network. A new ticket entry will be added to the persistent database storage to document the job activity.

Once we receive updated render images from the node, the ticket will be updated accordingly to properly refresh the current progress of the rendering batch.

If another node becomes available online, requesting a new ticket