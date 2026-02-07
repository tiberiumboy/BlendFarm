![[SettingPage.png]]
This page will list out all of the configuration this program can provide to the user. This allows to define custom blender installation path - render cache path to store all completed job renders, and local blender installation.

The Blender Installation Path outlines where the program can download and install blender from Blender's download page (https://download.blender.org/release) 

[Obsolete(This feature is only meant for the client to utilize, host have no purposes for this?)]
Blender File Cache Path is used for the client computer to utilize where to store and keep incoming project files from the server (host)

Render Cache Directory is used to store and keep all of the completed render images from the host and client node. When the client is completed with the render job, the client will send the completed image to the host, and the host will store the image to the provided path.

Blender Installation

Add from local storage lets the user of the machine to locally point to Blender installed path. (Dev - should we also distribute this to other client if os/arch matches?)

Install version expose all of blender's latest version available from the website, and install Blender automatically for the user.

The list below display all known blender installation the program can utilize and access. This list will only appear after validating Blender's executable path. (Feature - Allow user to run blender from here?)