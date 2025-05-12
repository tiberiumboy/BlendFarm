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

def useDevices(kind, allowGPU, allowCPU):
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
        d.use = (allowCPU and d.type == "CPU") or (allowGPU and d.type != "CPU")
        print(kind + " Device:", d["name"], d["use"])

#Renders provided settings with id to path
def renderWithSettings(renderSettings, frame):
    global scn

    # Scene parse
    scene = renderSettings["Scene"]
    if(scene is None):
        scene = ""
    if(scene != "" + scn.name != scene):
        print("Rendering specified scene " + scene + "\n")
        scn = bpy.data.scenes[scene]
        if(scn is None):
            raise Exception("Unknown Scene :" + scene)

    # set render format 
    renderFormat = renderSettings["RenderFormat"] or "PNG"
    scn.render.image_settings.file_format = renderFormat
        
    # Set threading
    scn.render.threads_mode = 'FIXED'
    scn.render.threads = max(cpu_count(), int(renderSettings["Cores"]))
    
    if (isPre3):
        scn.render.tile_x = int(renderSettings["TileWidth"])
        scn.render.tile_y = int(renderSettings["TileHeight"])
    
    # Set constraints
    scn.render.use_border = True
    scn.render.use_crop_to_border = renderSettings["Crop"]
    if not renderSettings["Crop"]:
        scn.render.film_transparent = True
        
    scn.render.border_min_x = float(renderSettings["Border"]["X"])
    scn.render.border_max_x = float(renderSettings["Border"]["X2"])
    scn.render.border_min_y = float(renderSettings["Border"]["Y"])
    scn.render.border_max_y = float(renderSettings["Border"]["Y2"])

    #Set Camera
    camera = renderSettings["Camera"]
    if(camera != None and camera != "" and bpy.data.objects[camera]):
        scn.camera = bpy.data.objects[camera]

    #Set Resolution
    scn.render.resolution_x = int(renderSettings["Width"])
    scn.render.resolution_y = int(renderSettings["Height"])
    scn.render.resolution_percentage = 100

    #Set Samples
    scn.cycles.samples = int(renderSettings["Samples"])
    scn.render.use_persistent_data = True

    # Set Frames Per Second
    fps = renderSettings["FPS"]
    if fps is not None and fps > 0:
        scn.render.fps = fps

    #Render 
    renderKind = renderSettings["RenderKind"]

    # This might get replaced
    engine = int(renderSettings["Engine"])
    
    scn.cycles.device = renderKind["Device"]
    useDevices(renderKind["Processor"], renderKind["UseGpu"], renderKind["UseCpu"])

    if(engine != 2): #Cycles/Eevee
        scn.cycles.device = renderKind["Device"]

    if(engine == 1): #Eevee
        # blender uses the new BLENDER_EEVEE_NEXT enum for blender4.2 and above.
        scn.render.engine = "BLENDER_EEVEE" if isPreEeveeNext else "BLENDER_EEVEE_NEXT"
    else:
        scn.render.engine = "CYCLES"
    
    # Set frame
    scn.frame_set(frame)
    
    # Set Output
    scn.render.filepath = renderSettings["Output"] + '/' + str(frame).zfill(5)
    id = str(renderSettings["TaskID"])

    # Render
    print("RENDER_START: " + id + "\n", flush=True)
    # TODO: Research what use_viewport does?
    bpy.ops.render.render(animation=False, write_still=True, use_viewport=False, layer="", scene=scene)
    print("SUCCESS: " + id + "\n", flush=True)

def runBatch():
    proxy = xmlrpc.client.ServerProxy("http://localhost:8081")
    renderSettings = None
    try:
        renderSettings = proxy.fetch_info(1)
    except Exception as e:
        print("EXCEPTION: Fail to call fetch_info over xml_rpc: " + str(e) + "\n")
        return
                
    # Loop over batches
    while True:
        try:
            frame = proxy.next_render_queue(1)
            renderWithSettings(renderSettings, frame)
        except Exception as e:
            print(e)
            break

    print("COMPLETED\n")

#Main
try:
    runBatch()
except Exception as e:
    print("EXCEPTION:" + str(e) + "\n")