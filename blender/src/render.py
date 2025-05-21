# Sybren mention that Cycle will perform better if the render was sent out as a batch instead of individual renders.
# TODO: See if there's a way to adjust blender render batch if possible?

#Start
import bpy # type: ignore
import xmlrpc.client
import json
from multiprocessing import cpu_count

isPre3 = bpy.app.version < (3,0,0)

def eprint(msg):
    print("EXCEPTION:" + str(msg) + "\n")

# hardware:[CPU,GPU,BOTH], kind: [NONE, CUDA, OPTIX, HIP, ONEAPI, (METAL?)]
# Eventually in the future we could distribute to a point of using certain GPU for certain render?
def configureSystemRenderDevices(kind, hardware):
    print("Setting up Cycles Render Devices")
    pref = bpy.context.preferences.addons["cycles"].preferences
    pref.compute_device_type = kind

    devices = None
    #For older Blender Builds
    if (isPre3):
        cuda_devices, opencl_devices = pref.get_devices()
        
        if(kind in ["CUDA","OPTIX"]):
            devices = cuda_devices
        else:
            devices = opencl_devices
    #For Blender Builds >= 3.0
    else:
        devices = pref.get_devices_for_type(pref.compute_device_type)
            
    for d in devices:
        # devices do not show GPU, instead they show what your GPU supports (CUDA for RTX)
        #               CPU                             GPU                                  ALL
        d.use = (d.type == hardware) or (d.type != 'CPU' and hardware == 'GPU') or ( hardware == "BOTH")

def setRenderSettings(scn, renderSetting, hardware):
    # this attribute only accepts 'CPU' or 'GPU' - only available in Cycles Render Engine
    scn.cycles.device = hardware

    #Set Samples
    scn.cycles.samples = int(renderSetting["sample"])
    scn.render.use_persistent_data = True

    # Set Frames Per Second
    fps = renderSetting["FPS"]
    if fps is not None and fps > 0:
        scn.render.fps = fps

    #Set Resolution
    scn.render.resolution_x = int(renderSetting["width"])
    scn.render.resolution_y = int(renderSetting["height"])
    scn.render.resolution_percentage = 100

    # Set borders
    border = renderSetting["border"]
    scn.render.border_min_x = float(border["X"])
    scn.render.border_max_x = float(border["X2"])
    scn.render.border_min_y = float(border["Y"])
    scn.render.border_max_y = float(border["Y2"])

# Setup blender configs
def setupBlenderSettings(scn, config):
    # Scene parse
    sceneInfo = config["SceneInfo"]
    
    #Set Camera
    camera = sceneInfo["camera"]
    if(camera != None and camera != "" and bpy.data.objects[camera]):
        scn.camera = bpy.data.objects[camera]
    
    # set scene render engine
    scn.render.engine = config["Engine"]

    # set render format 
    file_format = config["Format"]
    if(file_format is not None):
        scn.render.image_settings.file_format = file_format
        
    # Set threading
    threads = int(config["Cores"])
    scn.render.threads_mode = 'FIXED'
    scn.render.threads = max(cpu_count(), threads)
    
    # is this still possible? not sure if we still need this?
    if (isPre3):
        scn.render.tile_x = int(config["TileWidth"])
        scn.render.tile_y = int(config["TileHeight"])
    
    # Set constraints
    scn.render.use_border = True
    scn.render.use_crop_to_border = config["Crop"]
    if not config["Crop"]:
        scn.render.film_transparent = True
    
    hardware = config["HardwareMode"]
    # set render settings
    setRenderSettings(scn, sceneInfo["render_setting"], hardware)
    
    # Conifgure System Render Devices
    configureSystemRenderDevices(config["Processor"], hardware)

#Renders provided settings with id to path
def renderFrame(scn, config, scene, frame):
    # Set frame and output
    scn.frame_set(frame)
    scn.render.filepath = config["Output"] + '/' + str(frame).zfill(5)

    # Render
    id = str(config["TaskID"])
    print("RENDER_START: " + id + "\n", flush=True)

    # TODO: Research what use_viewport does?
    bpy.ops.render.render(animation=False, write_still=True, use_viewport=False, layer="", scene=scene)
    print("SUCCESS: " + id + "\n", flush=True)

def main():
    proxy = xmlrpc.client.ServerProxy("http://localhost:8081")
    config = None
    try:
        config = json.loads(proxy.fetch_info(1))  
    except Exception as e:
        eprint(e)
        return
    
    # Gather scene info
    scn = bpy.context.scene
    scene = config["SceneInfo"]["scene"]
    
    # set current scene
    if(scene is not None and scene != "" and scn.name != scene):
        print("LOG: Overriding default scene - using target scene: " + scene + "\n")
        scn = bpy.data.scenes[scene]
        if(scn is None):
            raise Exception("Scene name does not exist:" + scene)
    
    # configure the scene
    setupBlenderSettings(scn, config)
                
    # Loop over batches
    while True:
        try:
            frame = proxy.next_render_queue(1)
        except:
            break
        renderFrame(scn, config, scene, frame)

    print("COMPLETED")

main()