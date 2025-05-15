# TODO: Refactor this so it's less code to read through.
# Sybren mention that Cycle will perform better if the render was sent out as a batch instead of individual renders.
# TODO: See if there's a way to adjust blender render batch if possible?

#Start
import bpy # type: ignore
import xmlrpc.client
from multiprocessing import cpu_count

isPre3 = bpy.app.version < (3,0,0)
# Eventually this might get removed due to getting actual value from blend file instead
isPreEeveeNext = bpy.app.version < (4, 2, 0)

scn = bpy.context.scene

# change allowGPU/allowCPU to rely on hardwareMode (CPU|GPU|BOTH)

def useDevices(kind, hardware):
    scn.cycles.device = kind
    cyclesPref = bpy.context.preferences.addons["cycles"].preferences
    cyclesPref.compute_device_type = kind
    devices = None
    
    #For older Blender Builds
    if (isPre3):
        cuda_devices, opencl_devices = cyclesPref.get_devices()
        
        if(kind == "CUDA"):
            devices = cuda_devices
        elif(kind == "OPTIX"):
            devices = cuda_devices
        else:
            devices = opencl_devices
    #For Blender Builds >= 3.0
    else:
        # TODO: Run some unit test to see if this still works. This might break if someone tries to run blender > 3.0 and use CPU only
        if(kind != "CPU"):
            devices = cyclesPref.get_devices_for_type(kind)
            
        if(len(devices) == 0):
            raise Exception("No devices found for type " + kind + ", Unsupported hardware or platform?")
    
    for d in devices:
        d.use = (d.type == hardware or hardware == "BOTH") # or (allowGPU and d.type != "CPU")  // todo see if d.type is GPU?
        print("d.type:", d.type, hardware, d.use)
        print(kind + " Device:", d["name"], d["use"])

#Renders provided settings with id to path
def renderWithSettings(config, frame):
    global scn

    # Scene parse
    sceneInfo = config["SceneInfo"]
    scene = sceneInfo["scene"]
    renderSetting = sceneInfo["render_setting"]

    if(scene is None):
        scene = ""
    if(scene != "" + scn.name != scene):
        print("Rendering specified scene " + scene + "\n")
        scn = bpy.data.scenes[scene]
        if(scn is None):
            raise Exception("Unknown Scene :" + scene)

    # set render format 
    scn.render.image_settings.file_format = config["Format"] or "PNG"
        
    # Set threading
    scn.render.threads_mode = 'FIXED'
    scn.render.threads = max(cpu_count(), int(config["Cores"]))
    
    # is this still possible? not sure if we still need this?
    if (isPre3):
        scn.render.tile_x = int(config["TileWidth"])
        scn.render.tile_y = int(config["TileHeight"])
    
    # Set constraints
    scn.render.use_border = True
    scn.render.use_crop_to_border = config["Crop"]
    if not config["Crop"]:
        scn.render.film_transparent = True
        
    scn.render.border_min_x = float(sceneInfo["Border"]["X"])
    scn.render.border_max_x = float(sceneInfo["Border"]["X2"])
    scn.render.border_min_y = float(sceneInfo["Border"]["Y"])
    scn.render.border_max_y = float(sceneInfo["Border"]["Y2"])

    #Set Camera
    camera = sceneInfo["camera"]
    if(camera != None and camera != "" and bpy.data.objects[camera]):
        scn.camera = bpy.data.objects[camera]

    #Set Resolution
    scn.render.resolution_x = int(renderSetting["width"])
    scn.render.resolution_y = int(renderSetting["height"])
    scn.render.resolution_percentage = 100

    #Set Samples
    scn.cycles.samples = int(renderSetting["Sample"])
    scn.render.use_persistent_data = True

    # Set Frames Per Second
    fps = renderSetting["FPS"]
    if fps is not None and fps > 0:
        scn.render.fps = fps

    # This might get replaced
    engine = config["Engine"]
    processor = config["Processor"]
    hardware = config["HardwareMode"]

    useDevices(processor, hardware)

    if(engine == "BLENDER_EEVEE"): #Eevee
        # blender uses the new BLENDER_EEVEE_NEXT enum for blender4.2 and above.
        scn.render.engine = engine if isPreEeveeNext else "BLENDER_EEVEE_NEXT"
    else:
        scn.render.engine = "CYCLES"
    
    # Set frame
    scn.frame_set(frame)
    
    # Set Output
    scn.render.filepath = config["Output"] + '/' + str(frame).zfill(5)
    id = str(config["TaskID"])

    # Render
    print("RENDER_START: " + id + "\n", flush=True)
    # TODO: Research what use_viewport does?
    bpy.ops.render.render(animation=False, write_still=True, use_viewport=False, layer="", scene=scene)
    print("SUCCESS: " + id + "\n", flush=True)

def runBatch():
    proxy = xmlrpc.client.ServerProxy("http://localhost:8081")
    config = None
    try:
        config = proxy.fetch_info(1)
        print("Config:\n", config)   # testing out something here.
    except Exception as e:
        print("EXCEPTION: Fail to call fetch_info over xml_rpc: " + str(e) + "\n")
        return
                
    # Loop over batches
    while True:
        try:
            frame = proxy.next_render_queue(1)
            renderWithSettings(config, frame)
        except Exception as e:
            print(e)
            break

    print("COMPLETED")

#Main
try:
    runBatch()
except Exception as e:
    print("EXCEPTION:" + str(e) + "\n")