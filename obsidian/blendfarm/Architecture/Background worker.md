The background worker is design as a separate thread task to start the rendering task. 
Once a ticket is received (from source*), a new thread will spin up to ensure project file exist in the blend files directory (Under the schema of "blend_dir/job_id /file.blend"). 
Next, a call to blender manager to ensure we have the correct target blender version installed and ready to be used. It will first rely on available DHT services on the network to fetch compatible version first before downloading the raw source from the internet. 
Afterward, it will spin up a blender instances to render the provided frames window. 

From this background worker, a receiver channel will produce blender events to subscribers from another service.

