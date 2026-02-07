Seems like the code was not implemented to delete local content of blender file. 
We should provide a dialog asking user to disconnect blender link or delete local content where blender is store/installed.

Expected behaviour - when user deletes blender from the settings.rs, it should delete the blender content from the local machine and clear the row entry from settings page (Refresh/update?).

Actual behaviour - Program will crashed on macos - we need to verify that the path is correct and not linked to the executable inside appbundle