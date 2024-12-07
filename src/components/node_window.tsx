import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps, ComputerSpec } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";
import { listen } from "@tauri-apps/api/event";

export interface node {

}

export default function NodeWindow() {
  const [nodes, setNodes] = useState<RenderNodeProps[]>([]);

  //TODO: Do I need this? I use this to make node reveal to the GUI to say Hey I'm connecting... Be patient.
  listen<string>('node_discover', (event: any) => {
    let tmp = [...nodes];
    // once discover, parse the payload into the name field.
    let id = event.payload;
    let prop: RenderNodeProps = { name: id };   
    
    if (!tmp.includes(id)) {
      tmp.push(id);
    }
    setNodes(tmp);
  });

  listen('node_disconnect', (event: any) => {
    let tmp = [...nodes];
    let id = event.payload;
    tmp.filter((t) => t.name == id);
    setNodes(tmp);
  });

  // This is later fetch after the node sends the host information about the specs.
  listen('node_identity', (event: any) => {
    // here we'll parse teh information into json format
    // 0 is peer_id in base58, 1 is computer specs object
    let id = event.payload[0];
    // 1 is the computer spec payload
    let spec: ComputerSpec = event.payload[1];

    let node: RenderNodeProps = { name: id, spec };
    
    let tmp = [...nodes];
    tmp.filter((t) => t.name === id);
    tmp.push(node); 
    console.log(tmp);
    setNodes(tmp);
  })

  // TODO: Find a way to make this node selectable, and refresh the screen to display node property and information (E.g. Blender preview window, Activity monitor, specs, files completed, etc.)
  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <div className="group" id="RenderNodes">
          {nodes.map((node) =>
            <div>{RenderNode(node)}</div> 
          )}
        </div>
      </div>
    );
  }

  return (
    <div>
      {nodeWindow()}
    </div>
  );
}
