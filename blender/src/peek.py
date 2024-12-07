# Script used by LogicReinc.BlendFarm.Server for information extraction in Blender
# Assumes usage of structures from said assembly

#Start
import bpy # type: ignore
import json
import os

scn = bpy.context.scene

try:
    peekObj = dict(
        LastVersion = '.'.join(str(x) for x in bpy.data.version[:-1]) + ".0", # find a way to allow semvar version to accept a value higher than this patch.
        RenderWidth = scn.render.resolution_x,
        RenderHeight = scn.render.resolution_y,
        FrameStart = scn.frame_start,
        FrameEnd = scn.frame_end,
        FPS = scn.render.fps,
        Denoiser = scn.cycles.denoiser,
        Samples = scn.cycles.samples,
        Cameras = [],
        SelectedCamera = scn.camera.name,
        Scenes = [],
        SelectedScene = scn.name
    )
    for obj in scn.objects:
        if(obj.type == "CAMERA"):
            peekObj["Cameras"].append(obj.name)
            
    for scene in bpy.data.scenes:
        peekObj["Scenes"].append(scene.name)

    # how can I clear any message before this?
    os.system( 'cls' )
    print(json.dumps(peekObj)+"\n")

except Exception as e:
    print("EXCEPTION:" + str(e))