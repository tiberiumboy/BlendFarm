# NOTE: Sybren mention that Cycle will perform better if the render was sent out as
# a batch instead of individual renders. Consider using Range()
# TODO: What's the earliest python version blender supports? Wanted to make sure we are compilance with older version to use supported built-in library stacks.

import bpy # type: ignore
import json
import sys # used for argparse - does not work well with blender!
# from typing import Optional
# from dataclasses import dataclass
from multiprocessing import cpu_count

def eprint(msg):
    """Print exception tag message to console for program to intercept"""
    print(f"EXCEPTION: {msg}\n", flush=True)

def log(msg):
    """Print log tag message to console for program to intercept"""
    print(f"LOG: {msg}\n", flush=True)

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
def configure_system_render_devices(processor, hardware) -> None:
    """Setting up Cycle render devices"""
    pref = bpy.context.preferences.addons["cycles"].preferences
    pref.compute_device_type = processor
    devices = pref.get_devices_for_type(pref.compute_device_type)

    for d in devices:
        # devices do not show GPU, instead they show what your GPU supports (CUDA for RTX)
        #               CPU                             GPU                                  ALL
        d.use = (d.type == hardware) or (d.type != 'CPU' and hardware == 'GPU') or ( hardware == "BOTH")

def set_render_settings(scn, config) -> None:
    """Configure render settings from configs"""
    scene_info = config["SceneInfo"]
    render_setting = scene_info["render_setting"]

    #Set Camera
    camera = scene_info["camera"]
    if(camera is not None and bpy.data.objects[camera] is not None):
        scn.camera = bpy.data.objects[camera]

    # this attribute only accepts 'CPU' or 'GPU' - only available in Cycles Render Engine
    scn.cycles.device = config["HardwareMode"]

    # Conifgure System Render Devices
    configure_system_render_devices(config["Processor"], scn.cycles.device)

    #Set Samples
    scn.cycles.samples = render_setting["sample"]
    scn.render.use_persistent_data = True

    # Set Frames Per Second
    fps = render_setting["FPS"]
    if fps is not None and fps > 0:
        scn.render.fps = fps

    #Set Resolution
    scn.render.resolution_x = render_setting["width"]
    scn.render.resolution_y = render_setting["height"]
    scn.render.resolution_percentage = 100

    # Set borders
    border = render_setting["border"]
    scn.render.border_min_x = border["X"]
    scn.render.border_max_x = border["X2"]
    scn.render.border_min_y = border["Y"]
    scn.render.border_max_y = border["Y2"]

    # set render format 
    file_format = config["Format"]
    if file_format is not None:
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
def render_batch(scn, config):
    """Begin render a batch"""
    # Set frame and output
    # ref: https://docs.blender.org/api/current/bpy.types.Scene.html#bpy.types.Scene.frame_start 
    scn.frame_start = int(config["Start"])
    scn.frame_end = int(config["End"])

    # We must override the output path to a valid known location
    scn.render.filepath = config["Output"] + '''/#####'''

    # Render
    ticket_id = str(config["TicketID"])
    # TODO: How do I stream this? Why do I have to "flush"?
    print(f"RENDER_START: {ticket_id}\n", flush=True)
    # TODO: Research what use_viewport does? What about animation?
    bpy.ops.render.render(animation=True, write_still=True, use_viewport=False)
    # TODO: How do I stream this? Why do I have to "flush"?
    print(f"SUCCESS: {ticket_id}\n", flush=True)

def main(config) -> None:
    """Main entry point for render handler"""
    scn = bpy.context.scene
    set_render_settings(scn, config)
    render_batch(scn, config)

if __name__ == "__main__":
    # argparse.ArgumentParser does not work well with blender! Avoid using argparse!
    args = sys.argv
    try:
        content = args[args.index("-c")+1]
        config = json.loads(content)
        # config = json.loads(proxy.fetch_info(1))
    except Exception as e:
        print("Unable to parse content!:", e, flush=True)
        sys.exit(-1)

    try:
        main(config)
    except Exception as e:
        print("Received an error!:", e, flush=True)
        sys.exit(-1)

    sys.exit(0)
