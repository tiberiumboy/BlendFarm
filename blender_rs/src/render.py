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
    scn.render.engine = config["Engine"]
    
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
def renderFrame(scn, config, frame):
    # Set frame and output
    # TODO: Change frame to range instead and use the following api:
    # scn.frame_start = frame_start,
    # scn.frame_end = frame_end,
    scn.frame_set(frame)
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

def main(ip: str, port: int) -> None:
    # TODO: Consider sanitize ip first
    # Had connection refused?
    proxy = xmlrpc.client.ServerProxy("http://%s:%s" % (ip, port))
    
    # TODO: Cast as Config to enforce arguments sanitization
    config = None 
    try:
        print("About to fetch config", flush=True)
        config = json.loads(proxy.fetch_info(1))  
    except Exception as e:
        eprint(f"Failed to fetch config info! {e}")
        return
    
    # Gather scene info
    scn = bpy.context.scene
    
    # configure the scene
    # set scene if there's any
    # I don't see any reason why we should override the scene information here? 
    # Rely on the file and render what they provide us with. 
    # The file itself contains information to what scene to render from anyway?
    # scene = sceneInfo["scene"]
    # if(scene is not None and scene != "" and scn.name != scene):
    #     log("Overriding default scene - using target scene: " + scene + "\n")
    #     scn = bpy.data.scenes[scene]
    #     if(scn is None):
    #         raise Exception("Scene name does not exist:" + scene)

    
    # set render settings
    setRenderSettings(scn, config)
                
    # Loop over batches
    while True:
        try:
            # TODO: at a good time we can feed in as Optional[Single(int), Range(frame_start,frame_end)]
            frame = proxy.next_render_queue(1)
            if frame is None:
                break
            # TODO Change frame to range of frames
            renderFrame(scn, config, frame)
        except Exception as e:
            print(e)    # Wanted to see what the logs looks like so we can handle this better here
            break

if __name__ == "__main__":
    # argparse.ArgumentParser does not work well with blender! Avoid using argparse!
    try:
        args = sys.argv
        ip = args[args.index('-i')+1]
        port = args[args.index('-p')+1]
        main(ip, port)
    except Exception as e:
        print(e)
        sys.exit(1)
        