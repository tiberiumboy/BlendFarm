# TODO: Sybren mention that Cycle will perform better if the render was sent out as 
# a batch instead of individual renders. Consider using Range()
# TODO: See if there's a way to adjust blender render batch if possible?
# TODO: What's the earliest python version blender supports? Wanted to make sure we are compilance with older version to use supported built-in library stacks.

import bpy # type: ignore
import xmlrpc.client
import json
import sys # used for argparse - does not work well with blender!
# from typing import Optional
# from dataclasses import dataclass
from multiprocessing import cpu_count

def eprint(msg):
    print("EXCEPTION:" + str(msg) + "\n", flush=True)

def log(msg):
    print("LOG:" + str(msg) + "\n", flush=True)

# Feature thing, For now keep it dynamic.
# @dataclass
# class SceneInfo(object):
#     scene: Optional[str]

# @dataclass
# class Config(object):
#     scene_info: SceneInfo 

#     @classmethod
#     def from_json(cls, json_key):
#         file = json.load(open("h.json"))
#         return cls(**file[json_key])

# hardware:[CPU,GPU,BOTH], kind: [NONE, CUDA, OPTIX, HIP, ONEAPI, (METAL?)]
# Eventually in the future we could distribute to a point of using certain GPU for certain render?
def configureSystemRenderDevices(processor, hardware):
    # log("Setting up Cycles Render Devices")
    pref = bpy.context.preferences.addons["cycles"].preferences
    pref.compute_device_type = processor
    devices = pref.get_devices_for_type(pref.compute_device_type)
            
    for d in devices:
        # devices do not show GPU, instead they show what your GPU supports (CUDA for RTX)
        #               CPU                             GPU                                  ALL
        d.use = (d.type == hardware) or (d.type != 'CPU' and hardware == 'GPU') or ( hardware == "BOTH")

def setRenderSettings(scn, config):
    sceneInfo = config["SceneInfo"]
    renderSetting = sceneInfo["render_setting"] 

    #Set Camera
    camera = sceneInfo["camera"]
    if(camera is not None and bpy.data.objects[camera] is not None):
        scn.camera = bpy.data.objects[camera]
    
    # set scene render engine
    # *We should rely on the scene file engine configuration, rather than explicitly assigning before batch jobs.
    # scn.render.engine = config["Engine"]
    
    # this attribute only accepts 'CPU' or 'GPU' - only available in Cycles Render Engine
    scn.cycles.device = config["HardwareMode"]

    # Conifgure System Render Devices
    configureSystemRenderDevices(config["Processor"], scn.cycles.device)

    #Set Samples
    scn.cycles.samples = renderSetting["sample"]
    scn.render.use_persistent_data = True

    # Set Frames Per Second
    fps = renderSetting["FPS"]
    if fps is not None and fps > 0:
        scn.render.fps = fps

    #Set Resolution
    scn.render.resolution_x = renderSetting["width"]
    scn.render.resolution_y = renderSetting["height"]
    scn.render.resolution_percentage = 100

    # Set borders
    border = renderSetting["border"]
    scn.render.border_min_x = border["X"]
    scn.render.border_max_x = border["X2"]
    scn.render.border_min_y = border["Y"]
    scn.render.border_max_y = border["Y2"]

    # set render format 
    file_format = config["Format"]
    if(file_format is not None):
        scn.render.image_settings.file_format = file_format
        
    # Set threading
    threads = config["Cores"]
    scn.render.threads_mode = 'FIXED'
    scn.render.threads = max(cpu_count(), threads)
    
    # Set constraints
    scn.render.use_border = True
    scn.render.use_crop_to_border = config["Crop"]
    if not scn.render.use_crop_to_border:
        scn.render.film_transparent = True

#Renders provided settings with id to path
def renderFrame(scn, config):
    # Set frame and output
    scn.frame_start = config["start"],
    scn.frame_end = config["end"],
    
    # We must override the output path to a valid known location
    scn.render.filepath = config["Output"] + '/' + str(frame).zfill(5)

    # Render
    id = str(config["TaskID"])
    # TODO: How do I stream this? Why do I have to "flush"?
    print("RENDER_START: " + id + "\n", flush=True)
    # TODO: Research what use_viewport does? What about animation?
    bpy.ops.render.render(animation=True, write_still=True, use_viewport=False)
    # TODO: How do I stream this? Why do I have to "flush"?
    print("SUCCESS: " + id + "\n", flush=True)

def main(config) -> None:
    # proxy = xmlrpc.client.ServerProxy("http://%s:%s" % (ip, port))
    scn = bpy.context.scene
    setRenderSettings(scn, config)    
    renderFrame(scn, config)

if __name__ == "__main__":
    # argparse.ArgumentParser does not work well with blender! Avoid using argparse!
    args = sys.argv
    try:
        content = args[args.index("-c")+1]
        config = json.loads(content)
        # config = json.loads(proxy.fetch_info(1))  
        main(config)
    except Exception as e:
        print(e)
        sys.exit(-1)
    sys.exit(0)
        